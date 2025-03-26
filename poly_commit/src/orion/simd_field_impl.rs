use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use polynomials::{EqPolynomial, MultilinearExtension};
use transcript::Transcript;

use crate::{
    orion::{
        utils::{
            commit_encoded, lut_open_linear_combine, orion_mt_openings, pack_simd,
            simd_open_linear_combine,
        },
        OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    },
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

#[inline(always)]
pub fn orion_commit_simd_field<F, SimdF, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = {
        let num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let relative_pack_size = ComPackF::PACK_SIZE / SimdF::PACK_SIZE;
    assert_eq!(ComPackF::PACK_SIZE % SimdF::PACK_SIZE, 0);

    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    assert_eq!(poly.hypercube_size() % relative_pack_size, 0);
    let packed_evals = pack_simd::<F, SimdF, ComPackF>(poly.hypercube_basis_ref());

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

// NOTE: this implementation doesn't quite align with opening for
// multilinear polynomials over base field,
// as this directly plug into GKR argument system.
// In that context, there is no need to evaluate,
// as evaluation statement can be reduced on the verifier side.
#[inline(always)]
pub fn orion_open_simd_field<F, SimdF, EvalF, ComPackF, T>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> OrionProof<EvalF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let msg_size = {
        let num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars, point.len());

        let (_, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        msg_size
    };

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

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
            poly.hypercube_basis_ref(),
            &eq_col_coeffs,
            &mut eval_row,
            &random_col_coeffs,
            &mut proximity_rows,
        ),
        _ => simd_open_linear_combine(
            ComPackF::PACK_SIZE,
            poly.hypercube_basis_ref(),
            &eq_col_coeffs,
            &mut eval_row,
            &random_col_coeffs,
            &mut proximity_rows,
        ),
    }

    // NOTE: MT opening for point queries
    let query_openings = orion_mt_openings(pk, transcript, scratch_pad);

    OrionProof {
        eval_row,
        proximity_rows,
        query_openings,
    }
}
