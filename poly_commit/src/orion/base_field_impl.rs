use std::ops::Mul;

use arith::{Field, SimdField};
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{
    orion::{
        pcs_impl::{commit_encoded, orion_mt_openings},
        utils::transpose_in_place,
    },
    traits::TensorCodeIOPPCS,
    SubsetSumLUTs, PCS_SOUNDNESS_BITS,
};

use super::{OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad};

#[inline(always)]
fn transpose_and_pack<F, PackF>(evaluations: &mut [F], row_num: usize) -> Vec<PackF>
where
    F: Field,
    PackF: SimdField<Scalar = F>,
{
    // NOTE: pre transpose evaluations
    let mut scratch = vec![F::ZERO; evaluations.len()];
    transpose_in_place(evaluations, &mut scratch, row_num);
    drop(scratch);

    // NOTE: SIMD pack each row of transposed matrix
    evaluations
        .chunks(PackF::PACK_SIZE)
        .map(SimdField::pack)
        .collect()
}

pub fn orion_commit_base_field<F, ComPackF>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<F>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.get_num_vars());
    let packed_rows = row_num / ComPackF::PACK_SIZE;
    assert_eq!(row_num % ComPackF::PACK_SIZE, 0);

    let mut evals = poly.coeffs.clone();
    assert_eq!(evals.len() % ComPackF::PACK_SIZE, 0);

    let mut packed_evals: Vec<ComPackF> = transpose_and_pack(&mut evals, row_num);
    drop(evals);

    // NOTE: transpose back to rows of evaluations, but packed
    let mut scratch = vec![ComPackF::ZERO; packed_rows * msg_size];
    transpose_in_place(&mut packed_evals, &mut scratch, msg_size);
    drop(scratch);

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

pub fn orion_open_base_field<F, EvalF, ComPackF, OpenPackF, T>(
    pk: &OrionSRS,
    poly: &MultiLinearPoly<F>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> (EvalF, OrionProof<EvalF>)
where
    F: Field,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.get_num_vars());
    let num_of_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: transpose evaluations for linear combinations in evaulation/proximity tests
    let mut evals = poly.coeffs.clone();
    assert_eq!(evals.len() % OpenPackF::PACK_SIZE, 0);

    let packed_evals: Vec<OpenPackF> = transpose_and_pack(&mut evals, row_num);
    drop(evals);

    // NOTE: declare the look up tables for column sums
    let packed_rows = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::new(OpenPackF::PACK_SIZE, packed_rows);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    let mut eval_row = vec![EvalF::ZERO; msg_size];

    let eq_col_coeffs = EqPolynomial::build_eq_x_r(&point[num_of_vars_in_msg..]);
    luts.build(&eq_col_coeffs);

    packed_evals
        .chunks(packed_rows)
        .zip(eval_row.iter_mut())
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_test_num];

    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs = transcript.generate_challenge_field_elements(row_num);
        luts.build(&random_coeffs);

        packed_evals
            .chunks(packed_rows)
            .zip(row_buffer.iter_mut())
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));
    });
    drop(luts);

    // NOTE: working on evaluation on top of evaluation response
    let mut scratch = vec![EvalF::ZERO; msg_size];
    let eval = MultiLinearPoly::evaluate_with_buffer(
        &eval_row,
        &point[..num_of_vars_in_msg],
        &mut scratch,
    );
    drop(scratch);

    // NOTE: MT opening for point queries
    let leaf_range = row_num / tree::leaf_adic::<F>();
    let query_openings = orion_mt_openings(pk, leaf_range, transcript, scratch_pad);

    (
        eval,
        OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        },
    )
}
