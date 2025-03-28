use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use polynomials::{EqPolynomial, MultilinearExtension, RefMultiLinearPoly};
use transcript::Transcript;

use crate::{
    orion::{
        utils::{
            commit_encoded, lut_open_linear_combine, orion_mt_openings, pack_from_base,
            simd_open_linear_combine,
        },
        OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    },
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

#[inline(always)]
pub fn orion_commit_base_field<F, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<F>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<F>(poly.num_vars());
    let packed_rows = row_num / ComPackF::PACK_SIZE;
    assert_eq!(row_num % ComPackF::PACK_SIZE, 0);

    assert_eq!(poly.hypercube_size() % ComPackF::PACK_SIZE, 0);
    let packed_evals = pack_from_base::<F, ComPackF>(poly.hypercube_basis_ref());

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

#[inline(always)]
pub fn orion_open_base_field<F, EvalF, ComPackF, OpenPackF, T>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<F>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> (EvalF, OrionProof<EvalF>)
where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (_, msg_size) = OrionSRS::evals_shape::<F>(poly.num_vars());

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: pack evaluations for linear combinations in evaulation/proximity tests
    assert_eq!(poly.hypercube_size() % OpenPackF::PACK_SIZE, 0);
    let packed_evals: Vec<OpenPackF> = pack_from_base(poly.hypercube_basis_ref());

    // NOTE: pre-compute the eq linear combine coeffs for linear combination
    let eq_col_coeffs = {
        let mut eq_vars = point[..num_vars_in_com_simd].to_vec();
        eq_vars.extend_from_slice(&point[num_vars_in_com_simd + num_vars_in_msg..]);
        EqPolynomial::build_eq_x_r(&eq_vars)
    };

    // NOTE: pre-declare the spaces for returning evaluation and proximity queries
    let mut eval_row = vec![EvalF::ZERO; msg_size];

    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_test_num];

    let random_col_coeffs: Vec<_> = (0..proximity_test_num)
        .map(|_| {
            let rand = transcript.generate_challenge_field_elements(point.len() - num_vars_in_msg);
            EqPolynomial::build_eq_x_r(&rand)
        })
        .collect();

    match F::NAME {
        GF2::NAME => lut_open_linear_combine(
            ComPackF::PACK_SIZE,
            &packed_evals,
            &eq_col_coeffs,
            &mut eval_row,
            &random_col_coeffs,
            &mut proximity_rows,
        ),
        _ => simd_open_linear_combine(
            ComPackF::PACK_SIZE,
            &packed_evals,
            &eq_col_coeffs,
            &mut eval_row,
            &random_col_coeffs,
            &mut proximity_rows,
        ),
    }
    drop(packed_evals);

    // NOTE: working on evaluation on top of evaluation response
    let mut scratch = vec![EvalF::ZERO; msg_size];
    let eval = RefMultiLinearPoly::from_ref(&eval_row).evaluate_with_buffer(
        &point[num_vars_in_com_simd..num_vars_in_com_simd + num_vars_in_msg],
        &mut scratch,
    );
    drop(scratch);

    // NOTE: MT opening for point queries
    let query_openings = orion_mt_openings(pk, transcript, scratch_pad);

    (
        eval,
        OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        },
    )
}
