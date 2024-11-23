use std::marker::PhantomData;

use arith::{Field, FieldSerdeError, SimdField};
use thiserror::Error;

use crate::{traits::TensorCodeIOPPCS, StructuredReferenceString};

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

#[derive(Clone, Debug)]
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

impl StructuredReferenceString for OrionSRS {
    type PKey = OrionSRS;

    type VKey = OrionSRS;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), self.clone())
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

#[derive(Clone, Debug, Default)]
pub struct OrionProof<EvalF: Field> {
    pub eval_row: Vec<EvalF>,
    pub proximity_rows: Vec<Vec<EvalF>>,
    pub query_openings: Vec<tree::RangePath>,
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
