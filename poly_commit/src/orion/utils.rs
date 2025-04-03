use arith::{ExtensionField, Field, SimdField};
use itertools::izip;
use serdes::SerdeError;
use thiserror::Error;
use transcript::Transcript;
use tree::Node;

use crate::{
    orion::linear_code::{OrionCode, OrionCodeParameter},
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

/*
 * PCS ERROR AND RESULT SETUP
 */

#[derive(Debug, Error)]
pub enum OrionPCSError {
    #[error("Orion PCS linear code parameter unmatch error")]
    ParameterUnmatchError,

    #[error("field serde error")]
    SerializationError(#[from] SerdeError),
}

pub type OrionResult<T> = std::result::Result<T, OrionPCSError>;

/*
 * RELEVANT TYPES SETUP
 */

#[derive(Clone, Debug, Default)]
pub struct OrionSRS {
    pub num_vars: usize,
    pub code_instance: OrionCode,
}

impl TensorCodeIOPPCS for OrionSRS {
    fn codeword_len(&self) -> usize {
        self.code_instance.code_len()
    }

    fn minimum_hamming_weight(&self) -> f64 {
        self.code_instance.hamming_weight()
    }
}

impl OrionSRS {
    pub fn new<F: Field>(num_vars: usize, code_instance: OrionCode) -> OrionResult<Self> {
        let (_, msg_size) = Self::evals_shape::<F>(num_vars);
        if msg_size != code_instance.msg_len() {
            return Err(OrionPCSError::ParameterUnmatchError);
        }

        // NOTE: we just move the instance of code,
        // don't think the instance of expander code will be used elsewhere
        Ok(Self {
            num_vars,
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
            num_vars: num_variables,
            code_instance: OrionCode::new(code_param_instance, msg_size, &mut rng),
        }
    }
}

pub type OrionCommitment = Node;

#[derive(Clone, Debug, Default)]
pub struct OrionScratchPad {
    pub interleaved_alphabet_commitment: tree::Tree,
    pub merkle_cap: Vec<Node>,
}

#[derive(Clone, Debug, Default)]
pub struct OrionProof<EvalF: Field> {
    pub eval_row: Vec<EvalF>,
    pub proximity_rows: Vec<Vec<EvalF>>,
    pub query_openings: Vec<tree::RangePath>,
    pub merkle_cap: Vec<Node>,
}

#[inline(always)]
pub(crate) fn commit_encoded<PackF>(
    pk: &OrionSRS,
    packed_evals: &[PackF],
    scratch_pad: &mut OrionScratchPad,
    packed_rows: usize,
    msg_size: usize,
) -> OrionResult<OrionCommitment>
where
    PackF: SimdField,
{
    // NOTE: packed codeword buffer and encode over packed field
    let mut packed_interleaved_codewords = vec![PackF::ZERO; packed_rows * pk.codeword_len()];
    izip!(
        packed_evals.chunks(msg_size),
        packed_interleaved_codewords.chunks_mut(pk.codeword_len())
    )
    .try_for_each(|(evals, codeword)| pk.code_instance.encode_in_place(evals, codeword))?;

    // NOTE: transpose codeword s.t., the matrix has codewords being columns
    let mut interleaved_alphabets = vec![PackF::ZERO; packed_rows * pk.codeword_len()];
    transpose(
        &packed_interleaved_codewords,
        &mut interleaved_alphabets,
        packed_rows,
    );
    drop(packed_interleaved_codewords);

    // NOTE: commit the interleaved codeword
    // we just directly commit to the packed field elements to leaves
    // Also note, when codeword is not power of 2 length, pad to nearest po2
    // to commit by merkle tree
    if !interleaved_alphabets.len().is_power_of_two() {
        let aligned_po2_len = interleaved_alphabets.len().next_power_of_two();
        interleaved_alphabets.resize(aligned_po2_len, PackF::ZERO);
    }
    scratch_pad.interleaved_alphabet_commitment =
        tree::Tree::compact_new_with_packed_field_elems(interleaved_alphabets);

    scratch_pad.merkle_cap = vec![scratch_pad.interleaved_alphabet_commitment.root()];

    Ok(scratch_pad.interleaved_alphabet_commitment.root())
}

#[inline(always)]
pub(crate) fn orion_mt_openings<EvalF, T>(
    pk: &OrionSRS,
    transcript: &mut T,
    scratch_pad: &OrionScratchPad,
) -> Vec<tree::RangePath>
where
    EvalF: ExtensionField,
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
    merkle_cap: &[Node],
) -> bool {
    let world_size = merkle_cap.len();
    let leaves_in_range_opening = OrionSRS::LEAVES_IN_RANGE_OPENING * world_size;
    let indices_per_merkle_cap = vk.codeword_len().next_power_of_two() / world_size;

    izip!(query_indices, range_openings).all(|(&qi, range_path)| {
        let index = qi % vk.codeword_len();
        let merkle_cap_index = index / indices_per_merkle_cap;
        let in_sub_tree_index = index % indices_per_merkle_cap;

        range_path.verify(&merkle_cap[merkle_cap_index])
            && in_sub_tree_index == range_path.left / leaves_in_range_opening
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
pub(crate) fn transpose<F: Field>(mat: &[F], out: &mut [F], row_num: usize) {
    let col_num = mat.len() / row_num;
    let batch_size = cache_batch_size::<F>();
    let mut batch_srt = 0;

    mat.chunks(batch_size).for_each(|ith_batch| {
        let (mut src_y, mut src_x) = (batch_srt / col_num, batch_srt % col_num);

        ith_batch.iter().for_each(|elem_j| {
            let dst = src_y + src_x * row_num;
            out[dst] = *elem_j;

            src_x += 1;
            if src_x >= col_num {
                src_x = 0;
                src_y += 1;
            }
        });

        batch_srt += batch_size
    });
}

#[inline(always)]
pub(crate) fn transpose_in_place<F: Field>(mat: &mut [F], scratch: &mut [F], row_num: usize) {
    transpose(mat, scratch, row_num);
    mat.copy_from_slice(scratch);
}

#[inline(always)]
pub(crate) fn pack_from_base<F, PackF>(evaluations: &[F]) -> Vec<PackF>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: SIMD pack neighboring base field evals
    evaluations
        .chunks(PackF::PACK_SIZE)
        .map(SimdField::pack)
        .collect()
}

/*
 * LINEAR OPERATIONS FOR GF2 (LOOKUP TABLE BASED)
 */

pub struct SubsetSumLUTs<F: Field> {
    pub entry_bits: usize,
    pub tables: Vec<Vec<F>>,
}

impl<F: Field> SubsetSumLUTs<F> {
    #[inline(always)]
    pub fn new(entry_bits: usize, table_num: usize) -> Self {
        assert!(entry_bits > 0 && table_num > 0);

        Self {
            entry_bits,
            tables: vec![vec![F::ZERO; 1 << entry_bits]; table_num],
        }
    }

    #[inline(always)]
    pub fn build(&mut self, weights: &[F]) {
        assert_eq!(weights.len(), self.entry_bits * self.tables.len());

        self.tables.iter_mut().for_each(|lut| lut.fill(F::ZERO));

        // NOTE: we are assuming that the table is for {0, 1}-linear combination
        izip!(&mut self.tables, weights.chunks(self.entry_bits)).for_each(
            |(lut_i, sub_weights)| {
                sub_weights.iter().enumerate().for_each(|(i, weight_i)| {
                    let bit_mask = 1 << i;
                    lut_i.iter_mut().enumerate().for_each(|(bit_map, li)| {
                        if bit_map & bit_mask == bit_mask {
                            *li += weight_i;
                        }
                    });
                });
            },
        );
    }

    #[inline(always)]
    pub fn lookup_and_sum<EntryF>(&self, indices: &[EntryF]) -> F
    where
        EntryF: SimdField,
    {
        // NOTE: at least the entry field elem should have a matching field size
        // and the the entry field being a SIMD field with same packing size as
        // the bits for the table entry
        assert_eq!(EntryF::FIELD_SIZE, 1);
        assert_eq!(EntryF::PACK_SIZE, self.entry_bits);
        assert_eq!(indices.len(), self.tables.len());

        izip!(&self.tables, indices)
            .map(|(t_i, index)| t_i[index.as_u32_unchecked() as usize])
            .sum()
    }
}

#[inline(always)]
pub(crate) fn lut_open_linear_combine<F, EvalF, SimdF>(
    com_pack_size: usize,
    packed_evals: &[SimdF],
    eq_col_coeffs: &[EvalF],
    eval_row: &mut [EvalF],
    rand_col_coeffs: &[Vec<EvalF>],
    proximity_rows: &mut [Vec<EvalF>],
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    SimdF: SimdField<Scalar = F>,
{
    // NOTE: declare the look up tables for column sums
    let table_num = com_pack_size / SimdF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(SimdF::PACK_SIZE, table_num);
    assert_eq!(com_pack_size % SimdF::PACK_SIZE, 0);

    let combination_size = eq_col_coeffs.len();
    let packed_row_size = combination_size / com_pack_size;

    // NOTE: working on evaluation response of tensor code IOP based PCS
    izip!(
        eq_col_coeffs.chunks(com_pack_size),
        packed_evals.chunks(packed_evals.len() / packed_row_size)
    )
    .for_each(|(eq_chunk, packed_evals_chunk)| {
        luts.build(eq_chunk);

        izip!(packed_evals_chunk.chunks(table_num), &mut *eval_row)
            .for_each(|(p_col, eval)| *eval += luts.lookup_and_sum(p_col));
    });

    // NOTE: compose proximity response(s) of tensor code IOP based PCS
    izip!(proximity_rows, rand_col_coeffs).for_each(|(row_buffer, random_coeffs)| {
        izip!(
            random_coeffs.chunks(com_pack_size),
            packed_evals.chunks(packed_evals.len() / packed_row_size)
        )
        .for_each(|(random_chunk, packed_evals_chunk)| {
            luts.build(random_chunk);

            izip!(packed_evals_chunk.chunks(table_num), &mut *row_buffer)
                .for_each(|(p_col, prox)| *prox += luts.lookup_and_sum(p_col));
        });
    });
    drop(luts);
}

#[inline(always)]
pub(crate) fn lut_verify_alphabet_check<F, SimdF, ExtF>(
    codeword: &[ExtF],
    fixed_rl: &[ExtF],
    query_indices: &[usize],
    packed_interleaved_alphabets: &[Vec<SimdF>],
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ExtF: ExtensionField<BaseField = F>,
{
    // NOTE: build up lookup table
    let tables_num = fixed_rl.len() / SimdF::PACK_SIZE;
    assert_eq!(fixed_rl.len() % SimdF::PACK_SIZE, 0);
    let mut luts = SubsetSumLUTs::<ExtF>::new(SimdF::PACK_SIZE, tables_num);

    luts.build(fixed_rl);

    izip!(query_indices, packed_interleaved_alphabets).all(|(qi, interleaved_alphabet)| {
        let index = qi % codeword.len();
        let alphabet = luts.lookup_and_sum(interleaved_alphabet);
        alphabet == codeword[index]
    })
}

/*
 * LINEAR OPERATIONS FOR MERSENNE31 (SIMD BASED)
 */

#[inline(always)]
fn simd_ext_base_inner_prod<F, ExtF, SimdF>(
    simd_ext_limbs: &[SimdF],
    simd_base_elems: &[SimdF],
) -> ExtF
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ExtF: ExtensionField<BaseField = F>,
{
    assert_eq!(simd_ext_limbs.len(), simd_base_elems.len() * ExtF::DEGREE);

    let mut ext_limbs = vec![F::ZERO; ExtF::DEGREE];

    izip!(&mut ext_limbs, simd_ext_limbs.chunks(simd_base_elems.len())).for_each(
        |(e, simd_ext_limb)| {
            let simd_sum: SimdF = izip!(simd_ext_limb, simd_base_elems)
                .map(|(a, b)| *a * b)
                .sum();
            *e = simd_sum.horizontal_sum();
        },
    );

    ExtF::from_limbs(&ext_limbs)
}

// NOTE(HS) this is only a helper function for SIMD inner product between
// extension fields with SIMD base fields - motivation here is decompose extension fields
// into base field limbs, decompose them, then reSIMD pack them.
// This method is applied column-wise, i.e., the data size is the same as linear combination
// size, which should be in total 1024 bits at this point (2025/03/11).
#[inline(always)]
fn transpose_and_pack<F, SimdF>(evaluations: &[F], row_num: usize) -> Vec<SimdF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut evaluations_transposed = vec![F::ZERO; evaluations.len()];
    transpose(evaluations, &mut evaluations_transposed, row_num);

    // NOTE: SIMD pack each row of transposed matrix
    evaluations_transposed
        .chunks(SimdF::PACK_SIZE)
        .map(SimdField::pack)
        .collect()
}

#[inline(always)]
pub(crate) fn simd_open_linear_combine<F, EvalF, SimdF>(
    com_pack_size: usize,
    packed_evals: &[SimdF],
    eq_col_coeffs: &[EvalF],
    eval_row: &mut [EvalF],
    rand_col_coeffs: &[Vec<EvalF>],
    proximity_rows: &mut [Vec<EvalF>],
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    SimdF: SimdField<Scalar = F>,
{
    // NOTE: check SIMD inner product numbers for column sums
    let simd_inner_prods = com_pack_size / SimdF::PACK_SIZE;
    assert_eq!(com_pack_size % SimdF::PACK_SIZE, 0);

    let combination_size = eq_col_coeffs.len();
    let packed_row_size = combination_size / com_pack_size;

    // NOTE: working on evaluation response of tensor code IOP based PCS
    izip!(
        eq_col_coeffs.chunks(com_pack_size),
        packed_evals.chunks(packed_evals.len() / packed_row_size)
    )
    .for_each(|(eq_chunk, packed_evals_chunk)| {
        let eq_col_coeffs_limbs: Vec<_> = eq_chunk.iter().flat_map(|e| e.to_limbs()).collect();
        let eq_col_simd_limbs = transpose_and_pack(&eq_col_coeffs_limbs, com_pack_size);

        izip!(packed_evals_chunk.chunks(simd_inner_prods), &mut *eval_row).for_each(
            |(p_col, eval)| {
                *eval += simd_ext_base_inner_prod::<_, EvalF, _>(&eq_col_simd_limbs, p_col)
            },
        );
    });

    // NOTE: compose proximity response(s) of tensor code IOP based PCS
    izip!(proximity_rows, rand_col_coeffs).for_each(|(row_buffer, random_coeffs)| {
        izip!(
            random_coeffs.chunks(com_pack_size),
            packed_evals.chunks(packed_evals.len() / packed_row_size)
        )
        .for_each(|(rand_chunk, packed_evals_chunk)| {
            let rand_coeffs_limbs: Vec<_> = rand_chunk.iter().flat_map(|e| e.to_limbs()).collect();
            let rand_simd_limbs = transpose_and_pack(&rand_coeffs_limbs, com_pack_size);

            izip!(
                packed_evals_chunk.chunks(simd_inner_prods),
                &mut *row_buffer
            )
            .for_each(|(p_col, prox)| {
                *prox += simd_ext_base_inner_prod::<_, EvalF, _>(&rand_simd_limbs, p_col)
            });
        });
    });
}

#[inline(always)]
pub(crate) fn simd_verify_alphabet_check<F, SimdF, ExtF>(
    codeword: &[ExtF],
    fixed_rl: &[ExtF],
    query_indices: &[usize],
    packed_interleaved_alphabets: &[Vec<SimdF>],
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ExtF: ExtensionField<BaseField = F>,
{
    // NOTE: check SIMD inner product numbers for column sums
    assert_eq!(fixed_rl.len() % SimdF::PACK_SIZE, 0);

    let rl_limbs: Vec<_> = fixed_rl.iter().flat_map(|e| e.to_limbs()).collect();
    let rl_simd_limbs: Vec<SimdF> = transpose_and_pack(&rl_limbs, fixed_rl.len());

    izip!(query_indices, packed_interleaved_alphabets).all(|(qi, interleaved_alphabet)| {
        let index = qi % codeword.len();
        let alphabet: ExtF = simd_ext_base_inner_prod(&rl_simd_limbs, interleaved_alphabet);
        alphabet == codeword[index]
    })
}

#[cfg(test)]
mod tests {
    use arith::{Field, SimdField};
    use ark_std::test_rng;
    use gf2::{GF2x8, GF2};
    use gf2_128::{GF2_128x8, GF2_128};
    use itertools::izip;

    use crate::orion::utils::SubsetSumLUTs;

    #[test]
    fn test_lut_simd_inner_prod_consistency() {
        let mut rng = test_rng();

        let weights: Vec<_> = (0..8).map(|_| GF2_128::random_unsafe(&mut rng)).collect();
        let bases: Vec<_> = (0..8).map(|_| GF2::random_unsafe(&mut rng)).collect();

        let simd_weights = GF2_128x8::pack(&weights);
        let simd_bases = GF2x8::pack(&bases);

        let expected_simd_inner_prod: GF2_128 = (simd_weights * simd_bases).horizontal_sum();

        let expected_vanilla_inner_prod: GF2_128 =
            izip!(&weights, &bases).map(|(w, b)| *w * *b).sum();

        assert_eq!(expected_simd_inner_prod, expected_vanilla_inner_prod);

        let mut table = SubsetSumLUTs::new(8, 1);
        table.build(&weights);

        let actual_lut_inner_prod = table.lookup_and_sum(&[simd_bases]);

        assert_eq!(expected_simd_inner_prod, actual_lut_inner_prod)
    }
}
