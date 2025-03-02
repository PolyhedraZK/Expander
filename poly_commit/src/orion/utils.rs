use std::marker::PhantomData;

use arith::{ExtensionField, Field, FieldSerde, FieldSerdeError, SimdField};
use itertools::izip;
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

pub type OrionCommitment = tree::Node;

#[derive(Clone, Debug, Default)]
pub struct OrionScratchPad<F, ComPackF>
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    pub interleaved_alphabet_commitment: tree::Tree,

    _phantom: PhantomData<ComPackF>,
}

unsafe impl<F: Field, ComPackF: SimdField<Scalar = F>> Send for OrionScratchPad<F, ComPackF> {}

impl<F: Field, ComPackF: SimdField<Scalar = F>> FieldSerde for OrionScratchPad<F, ComPackF> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, writer: W) -> arith::FieldSerdeResult<()> {
        self.interleaved_alphabet_commitment.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> arith::FieldSerdeResult<Self> {
        let interleaved_alphabet_commitment = tree::Tree::deserialize_from(reader)?;

        Ok(Self {
            interleaved_alphabet_commitment,
            _phantom: PhantomData,
        })
    }
}

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
    izip!(
        packed_evals.chunks(msg_size),
        packed_interleaved_codewords.chunks_mut(pk.codeword_len())
    )
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
    izip!(query_indices, range_openings).all(|(&qi, range_path)| {
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

#[inline(always)]
pub(crate) fn transpose_and_pack<F, SimdF>(evaluations: &mut [F], row_num: usize) -> Vec<SimdF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![F::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    evaluations
        .chunks(SimdF::PACK_SIZE)
        .map(SimdField::pack)
        .collect()
}

#[inline(always)]
pub(crate) fn transpose_and_pack_simd<F, SimdF, PackF>(
    evaluations: &mut [SimdF],
    row_num: usize,
) -> Vec<PackF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![SimdF::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    let relative_pack_size = PackF::PACK_SIZE / SimdF::PACK_SIZE;
    evaluations
        .chunks(relative_pack_size)
        .map(PackF::pack_from_simd)
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
        izip!(&mut self.tables, weights.chunks(self.entry_bits)).for_each(
            |(lut_i, sub_weights)| {
                sub_weights.iter().enumerate().for_each(|(i, weight_i)| {
                    let bit_mask = 1 << (self.entry_bits - i - 1);
                    lut_i.iter_mut().enumerate().for_each(|(bit_map, li)| {
                        if bit_map & bit_mask == bit_mask {
                            *li += weight_i;
                        }
                    });
                });
            },
        );
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

        izip!(&self.tables, indices)
            .map(|(t_i, index)| t_i[index.as_u32_unchecked() as usize])
            .sum()
    }
}

#[inline(always)]
pub(crate) fn lut_open_linear_combine<F, EvalF, SimdF, T>(
    row_num: usize,
    packed_evals: &[SimdF],
    eq_col_coeffs: &[EvalF],
    eval_row: &mut [EvalF],
    proximity_rows: &mut [Vec<EvalF>],
    transcript: &mut T,
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    SimdF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    // NOTE: declare the look up tables for column sums
    let table_num = row_num / SimdF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(SimdF::PACK_SIZE, table_num);
    assert_eq!(row_num % SimdF::PACK_SIZE, 0);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    luts.build(eq_col_coeffs);

    izip!(packed_evals.chunks(table_num), eval_row)
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        luts.build(&random_coeffs);

        izip!(packed_evals.chunks(table_num), row_buffer)
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));
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
pub(crate) fn simd_inner_product<F, SimdF>(lhs: &[SimdF], rhs: &[SimdF]) -> F
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
{
    assert_eq!(lhs.len(), rhs.len());

    let simd_sum: SimdF = izip!(lhs, rhs).map(|(a, b)| *a * b).sum();

    simd_sum.unpack().iter().sum()
}

#[inline(always)]
pub(crate) fn simd_ext_base_inner_prod<F, ExtF, SimdF>(
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

    izip!(&mut ext_limbs, simd_ext_limbs.chunks(simd_base_elems.len()))
        .for_each(|(e, simd_ext_limb)| *e = simd_inner_product(simd_ext_limb, simd_base_elems));

    ExtF::from_limbs(&ext_limbs)
}

#[inline(always)]
pub(crate) fn simd_open_linear_combine<F, EvalF, SimdF, T>(
    row_num: usize,
    packed_evals: &[SimdF],
    eq_col_coeffs: &[EvalF],
    eval_row: &mut [EvalF],
    proximity_rows: &mut [Vec<EvalF>],
    transcript: &mut T,
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    SimdF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    // NOTE: check SIMD inner product numbers for column sums
    let simd_inner_prods = row_num / SimdF::PACK_SIZE;
    assert_eq!(row_num % SimdF::PACK_SIZE, 0);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    let mut eq_col_coeffs_limbs: Vec<_> = eq_col_coeffs.iter().flat_map(|e| e.to_limbs()).collect();
    let eq_col_simd_limbs: Vec<_> = transpose_and_pack(&mut eq_col_coeffs_limbs, row_num);

    izip!(packed_evals.chunks(simd_inner_prods), eval_row)
        .for_each(|(p_col, res)| *res = simd_ext_base_inner_prod(&eq_col_simd_limbs, p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        let mut proximity_limbs: Vec<_> = random_coeffs.iter().flat_map(|e| e.to_limbs()).collect();
        let proximity_simd_limbs: Vec<_> = transpose_and_pack(&mut proximity_limbs, row_num);

        izip!(packed_evals.chunks(simd_inner_prods), row_buffer)
            .for_each(|(p_col, res)| *res = simd_ext_base_inner_prod(&proximity_simd_limbs, p_col));
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

    let mut rl_limbs: Vec<_> = fixed_rl.iter().flat_map(|e| e.to_limbs()).collect();
    let rl_simd_limbs: Vec<SimdF> = transpose_and_pack(&mut rl_limbs, fixed_rl.len());

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

    use super::SubsetSumLUTs;

    #[test]
    fn test_lut_simd_inner_prod_consistency() {
        let mut rng = test_rng();

        let weights: Vec<_> = (0..8).map(|_| GF2_128::random_unsafe(&mut rng)).collect();
        let bases: Vec<_> = (0..8).map(|_| GF2::random_unsafe(&mut rng)).collect();

        let simd_weights = GF2_128x8::pack(&weights);
        let simd_bases = GF2x8::pack(&bases);

        let expected_simd_inner_prod: GF2_128 = (simd_weights * simd_bases).unpack().iter().sum();

        let expected_vanilla_inner_prod: GF2_128 =
            izip!(&weights, &bases).map(|(w, b)| *w * *b).sum();

        assert_eq!(expected_simd_inner_prod, expected_vanilla_inner_prod);

        let mut table = SubsetSumLUTs::new(8, 1);
        table.build(&weights);

        let actual_lut_inner_prod = table.lookup_and_sum(&[simd_bases]);

        assert_eq!(expected_simd_inner_prod, actual_lut_inner_prod)
    }
}
