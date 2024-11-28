use std::marker::PhantomData;

use arith::{ExtensionField, Field, FieldSerdeError, SimdField};
use thiserror::Error;
use transcript::Transcript;

use crate::{traits::TensorCodeIOPPCS, PCS_SOUNDNESS_BITS};

use super::linear_code::{OrionCode, OrionCodeParameter};

/*
 * PCS ERROR AND RESULT SETUP
 */

#[derive(Debug, Error)]
pub enum OrionPCSError {
    #[error("Orion PCS linear code parameter unmatch error")]
    ParameterUnmatchError,

    #[error("field serde error")]
    SerializationError(#[from] FieldSerdeError),
}

pub type OrionResult<T> = std::result::Result<T, OrionPCSError>;

/*
 * RELEVANT TYPES SETUP
 */

#[derive(Clone, Debug, Default)]
pub struct OrionSRS {
    pub num_variables: usize,
    pub code_instance: OrionCode,
}

impl TensorCodeIOPPCS for OrionSRS {
    fn codeword_len(&self) -> usize {
        self.code_instance.code_len()
    }

    fn hamming_weight(&self) -> f64 {
        self.code_instance.hamming_weight()
    }
}

impl OrionSRS {
    pub fn new<F: Field>(num_variables: usize, code_instance: OrionCode) -> OrionResult<Self> {
        let (_, msg_size) = Self::evals_shape::<F>(num_variables);
        if msg_size != code_instance.msg_len() {
            return Err(OrionPCSError::ParameterUnmatchError);
        }

        // NOTE: we just move the instance of code,
        // don't think the instance of expander code will be used elsewhere
        Ok(Self {
            num_variables,
            code_instance,
        })
    }

    pub fn from_random<F: Field>(
        num_variables: usize,
        code_param_instance: OrionCodeParameter,
        mut rng: impl rand::RngCore,
    ) -> Self {
        let (_, msg_size) = Self::evals_shape::<F>(num_variables);

        Self {
            num_variables,
            code_instance: OrionCode::new(code_param_instance, msg_size, &mut rng),
        }
    }
}

pub type OrionCommitment = tree::Node;

// TODO: maybe prepare memory API for transpose in commit and open?
#[derive(Clone, Debug, Default)]
pub struct OrionScratchPad<F, ComPackF>
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    pub interleaved_alphabet_commitment: tree::Tree,
    pub _phantom: PhantomData<ComPackF>,
}

unsafe impl<F: Field, ComPackF: SimdField<Scalar = F>> Send for OrionScratchPad<F, ComPackF> {}

#[derive(Clone, Debug, Default)]
pub struct OrionProof<EvalF: Field> {
    pub eval_row: Vec<EvalF>,
    pub proximity_rows: Vec<Vec<EvalF>>,
    pub query_openings: Vec<tree::RangePath>,
}

#[inline(always)]
pub(crate) fn commit_encoded<F, PackF>(
    pk: &OrionSRS,
    packed_evals: &[PackF],
    scratch_pad: &mut OrionScratchPad<F, PackF>,
    packed_rows: usize,
    msg_size: usize,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: packed codeword buffer and encode over packed field
    let mut packed_interleaved_codewords = vec![PackF::ZERO; packed_rows * pk.codeword_len()];
    packed_evals
        .chunks(msg_size)
        .zip(packed_interleaved_codewords.chunks_mut(pk.codeword_len()))
        .try_for_each(|(evals, codeword)| pk.code_instance.encode_in_place(evals, codeword))?;

    // NOTE: transpose codeword s.t., the matrix has codewords being columns
    let mut scratch = vec![PackF::ZERO; packed_rows * pk.codeword_len()];
    transpose_in_place(&mut packed_interleaved_codewords, &mut scratch, packed_rows);
    drop(scratch);

    // NOTE: commit the interleaved codeword
    // we just directly commit to the packed field elements to leaves
    // Also note, when codeword is not power of 2 length, pad to nearest po2
    // to commit by merkle tree
    if !packed_interleaved_codewords.len().is_power_of_two() {
        let aligned_po2_len = packed_interleaved_codewords.len().next_power_of_two();
        packed_interleaved_codewords.resize(aligned_po2_len, PackF::ZERO);
    }
    scratch_pad.interleaved_alphabet_commitment =
        tree::Tree::compact_new_with_packed_field_elems::<F, PackF>(packed_interleaved_codewords);

    Ok(scratch_pad.interleaved_alphabet_commitment.root())
}

#[inline(always)]
pub(crate) fn orion_mt_openings<F, EvalF, ComPackF, T>(
    pk: &OrionSRS,
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> Vec<tree::RangePath>
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let leaves_in_range_opening = OrionSRS::LEAVES_IN_RANGE_OPENING;

    // NOTE: MT opening for point queries
    let query_num = pk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);
    query_indices
        .iter()
        .map(|qi| {
            let index = *qi % pk.codeword_len();
            let left = index * leaves_in_range_opening;
            let right = left + leaves_in_range_opening - 1;

            scratch_pad
                .interleaved_alphabet_commitment
                .range_query(left, right)
        })
        .collect()
}

#[inline(always)]
pub(crate) fn orion_mt_verify(
    vk: &OrionSRS,
    query_indices: &[usize],
    range_openings: &[tree::RangePath],
    root: &OrionCommitment,
) -> bool {
    let leaves_in_range_opening = OrionSRS::LEAVES_IN_RANGE_OPENING;
    query_indices
        .iter()
        .zip(range_openings.iter())
        .all(|(&qi, range_path)| {
            let index = qi % vk.codeword_len();
            range_path.verify(root) && index == range_path.left / leaves_in_range_opening
        })
}

/*
 * IMPLEMENTATIONS FOR MATRIX TRANSPOSE
 */

pub(crate) const fn cache_batch_size<F: Sized>() -> usize {
    const CACHE_SIZE: usize = 1 << 16;
    CACHE_SIZE / size_of::<F>()
}

#[inline(always)]
pub(crate) fn transpose_in_place<F: Field>(mat: &mut [F], scratch: &mut [F], row_num: usize) {
    let col_num = mat.len() / row_num;
    let batch_size = cache_batch_size::<F>();

    mat.chunks(batch_size)
        .enumerate()
        .for_each(|(i, ith_batch)| {
            let batch_srt = batch_size * i;

            ith_batch.iter().enumerate().for_each(|(j, &elem_j)| {
                let src = batch_srt + j;
                let dst = (src / col_num) + (src % col_num) * row_num;

                scratch[dst] = elem_j;
            })
        });

    mat.copy_from_slice(scratch);
}

/*
 * LINEAR OPERATIONS
 */

pub struct SubsetSumLUTs<F: Field> {
    pub entry_bits: usize,
    pub tables: Vec<Vec<F>>,
}

impl<F: Field> SubsetSumLUTs<F> {
    #[inline]
    pub fn new(entry_bits: usize, table_num: usize) -> Self {
        assert!(entry_bits > 0 && table_num > 0);

        Self {
            entry_bits,
            tables: vec![vec![F::ZERO; 1 << entry_bits]; table_num],
        }
    }

    #[inline]
    pub fn build(&mut self, weights: &[F]) {
        assert_eq!(weights.len() % self.entry_bits, 0);
        assert_eq!(weights.len() / self.entry_bits, self.tables.len());

        self.tables.iter_mut().for_each(|lut| lut.fill(F::ZERO));

        // NOTE: we are assuming that the table is for {0, 1}-linear combination
        self.tables
            .iter_mut()
            .zip(weights.chunks(self.entry_bits))
            .for_each(|(lut_i, sub_weights)| {
                sub_weights.iter().enumerate().for_each(|(i, weight_i)| {
                    let bit_mask = 1 << (self.entry_bits - i - 1);
                    lut_i.iter_mut().enumerate().for_each(|(bit_map, li)| {
                        if bit_map & bit_mask == bit_mask {
                            *li += weight_i;
                        }
                    });
                });
            });
    }

    #[inline]
    pub fn lookup_and_sum<BitF, EntryF>(&self, indices: &[EntryF]) -> F
    where
        BitF: Field,
        EntryF: SimdField<Scalar = BitF>,
    {
        // NOTE: at least the entry field elem should have a matching field size
        // and the the entry field being a SIMD field with same packing size as
        // the bits for the table entry
        assert_eq!(EntryF::FIELD_SIZE, 1);
        assert_eq!(EntryF::PACK_SIZE, self.entry_bits);
        assert_eq!(indices.len(), self.tables.len());

        self.tables
            .iter()
            .zip(indices.iter())
            .map(|(t_i, index)| t_i[index.as_u32_unchecked() as usize])
            .sum()
    }
}
