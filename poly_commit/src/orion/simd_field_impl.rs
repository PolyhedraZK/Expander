use std::ops::Mul;

use arith::{Field, SimdField};
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{
    orion::{
        utils::{commit_encoded, orion_mt_openings, transpose_in_place},
        OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    },
    traits::TensorCodeIOPPCS,
    SubsetSumLUTs, PCS_SOUNDNESS_BITS,
};

#[inline(always)]
fn transpose_and_pack_simd<F, CircuitF, PackF>(
    evaluations: &mut [CircuitF],
    row_num: usize,
) -> Vec<PackF>
where
    F: Field,
    CircuitF: SimdField<Scalar = F>,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![CircuitF::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    let relative_pack_size = PackF::PACK_SIZE / CircuitF::PACK_SIZE;
    evaluations
        .chunks(relative_pack_size)
        .map(SimdField::pack_from_simd)
        .collect()
}

pub fn orion_commit_simd_field<F, CircuitF, ComPackF>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<CircuitF>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    CircuitF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<CircuitF>(poly.get_num_vars());
    let relative_pack_size = ComPackF::PACK_SIZE / CircuitF::PACK_SIZE;
    assert_eq!(ComPackF::PACK_SIZE % CircuitF::PACK_SIZE, 0);

    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    let mut evals = poly.coeffs.clone();
    assert_eq!(evals.len() % relative_pack_size, 0);

    let mut packed_evals = transpose_and_pack_simd::<F, CircuitF, ComPackF>(&mut evals, row_num);
    drop(evals);

    // NOTE: transpose back to rows of evaluations, but packed
    let mut scratch = vec![ComPackF::ZERO; packed_rows * msg_size];
    transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
    drop(scratch);

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

#[inline(always)]
fn transpose_and_shuffle_simd<F, CircuitF, PackF>(
    evaluations: &mut [CircuitF],
    row_num: usize,
) -> Vec<PackF>
where
    F: Field,
    CircuitF: SimdField<Scalar = F>,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![CircuitF::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: reshuffle the transposed matrix, from SIMD over row to SIMD over col
    let mut scratch = vec![F::ZERO; CircuitF::PACK_SIZE * PackF::PACK_SIZE];
    evaluations
        .chunks(PackF::PACK_SIZE)
        .flat_map(|circuit_simd_chunk| -> Vec<PackF> {
            let mut temp: Vec<F> = circuit_simd_chunk.iter().flat_map(|c| c.unpack()).collect();
            transpose_in_place(&mut temp, &mut scratch, PackF::PACK_SIZE);
            temp.chunks(PackF::PACK_SIZE).map(PackF::pack).collect()
        })
        .collect()
}

// NOTE: this implementation doesn't quite align with opening for
// multilinear polynomials over base field,
// as this directly plug into GKR argument system.
// In that context, there is no need to evaluate,
// as evaluation statement can be reduced on the verifier side.
pub fn orion_open_simd_field<F, CircuitF, EvalF, CircuitEvalF, ComPackF, OpenPackF, T>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<CircuitF>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionProof<CircuitEvalF>
where
    F: Field,
    CircuitF: SimdField<Scalar = F>,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    CircuitEvalF: SimdField<Scalar = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    assert_eq!(CircuitF::PACK_SIZE, CircuitEvalF::PACK_SIZE);

    let (row_num, msg_size) = OrionSRS::evals_shape::<CircuitF>(poly.get_num_vars());
    let num_vars_in_row = row_num.ilog2() as usize;

    // NOTE: transpose and shuffle evaluations (repack evaluations in another direction)
    // for linear combinations in evaulation/proximity tests
    let mut evals = poly.coeffs.clone();
    assert_eq!(evals.len() * CircuitF::PACK_SIZE % OpenPackF::PACK_SIZE, 0);

    let packed_shuffled_evals: Vec<OpenPackF> = transpose_and_shuffle_simd(&mut evals, row_num);
    drop(evals);

    // NOTE: declare the look up tables for column sums
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<EvalF>::new(OpenPackF::PACK_SIZE, tables_num);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    let mut eval_row = vec![CircuitEvalF::ZERO; msg_size];

    let eq_col_coeffs = EqPolynomial::build_eq_x_r(&point[point.len() - num_vars_in_row..]);
    luts.build(&eq_col_coeffs);

    packed_shuffled_evals
        .chunks(tables_num * CircuitEvalF::PACK_SIZE)
        .zip(eval_row.iter_mut())
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum_simd(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![CircuitEvalF::ZERO; msg_size]; proximity_test_num];

    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        luts.build(&random_coeffs);

        packed_shuffled_evals
            .chunks(tables_num * CircuitEvalF::PACK_SIZE)
            .zip(row_buffer.iter_mut())
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum_simd(p_col));
    });
    drop(luts);

    // NOTE: MT opening for point queries
    let query_openings = orion_mt_openings(pk, transcript, scratch_pad);

    OrionProof {
        eval_row,
        proximity_rows,
        query_openings,
    }
}
