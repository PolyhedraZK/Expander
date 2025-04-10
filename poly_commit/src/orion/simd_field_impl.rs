use std::iter;

use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use gkr_engine::Transcript;
use itertools::izip;
use polynomials::{EqPolynomial, MultilinearExtension, RefMultiLinearPoly};

use crate::{
    orion::{utils::*, OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad},
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

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
    let packed_evals_ref = unsafe {
        let ptr = poly.hypercube_basis_ref().as_ptr();
        let len = poly.hypercube_size() / relative_pack_size;
        std::slice::from_raw_parts(ptr as *const ComPackF, len)
    };

    commit_encoded(pk, packed_evals_ref, scratch_pad, packed_rows, msg_size)
}

// NOTE: this implementation doesn't quite align with opening for
// multilinear polynomials over base field,
// as this directly plug into GKR argument system.
// In that context, there is no need to evaluate,
// as evaluation statement can be reduced on the verifier side.
pub fn orion_open_simd_field<F, SimdF, EvalF, ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    point: &[EvalF],
    transcript: &mut impl Transcript<EvalF>,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> OrionProof<EvalF>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
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

    match F::NAME {
        GF2::NAME => lut_open_linear_combine(
            ComPackF::PACK_SIZE,
            poly.hypercube_basis_ref(),
            &eq_col_coeffs,
            &mut eval_row,
            &mut proximity_rows,
            transcript,
        ),
        _ => simd_open_linear_combine(
            ComPackF::PACK_SIZE,
            poly.hypercube_basis_ref(),
            &eq_col_coeffs,
            &mut eval_row,
            &mut proximity_rows,
            transcript,
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

pub fn orion_verify_simd_field<F, SimdF, EvalF, ComPackF>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &[EvalF],
    evaluation: EvalF,
    transcript: &mut impl Transcript<EvalF>,
    proof: &OrionProof<EvalF>,
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = {
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(point.len());
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: working on evaluation response, evaluate the rest of the response
    let mut scratch = vec![EvalF::ZERO; msg_size];
    let final_eval = RefMultiLinearPoly::from_ref(&proof.eval_row).evaluate_with_buffer(
        &point[num_vars_in_com_simd..num_vars_in_com_simd + num_vars_in_msg],
        &mut scratch,
    );

    if final_eval != evaluation {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let random_linear_combinations: Vec<Vec<EvalF>> = (0..proximity_reps)
        .map(|_| transcript.generate_challenge_field_elements(row_num * SimdF::PACK_SIZE))
        .collect();

    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);

    // NOTE: check consistency in MT in the opening trees and against the commitment tree
    if !orion_mt_verify(vk, &query_indices, &proof.query_openings, commitment) {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but reshuffle the packed elements into another direction
    let packed_interleaved_alphabets: Vec<Vec<SimdF>> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            let elts = c.unpack_field_elems::<F, ComPackF>();
            elts.chunks(SimdF::PACK_SIZE).map(SimdF::pack).collect()
        })
        .collect();

    let eq_col_coeffs = {
        let mut eq_vars = point[..num_vars_in_com_simd].to_vec();
        eq_vars.extend_from_slice(&point[num_vars_in_com_simd + num_vars_in_msg..]);
        EqPolynomial::build_eq_x_r(&eq_vars)
    };

    izip!(&random_linear_combinations, &proof.proximity_rows)
        .chain(iter::once((&eq_col_coeffs, &proof.eval_row)))
        .all(|(rl, msg)| {
            let codeword = match vk.code_instance.encode(msg) {
                Ok(c) => c,
                _ => return false,
            };

            match F::NAME {
                GF2::NAME => lut_verify_alphabet_check(
                    &codeword,
                    rl,
                    &query_indices,
                    &packed_interleaved_alphabets,
                ),
                _ => simd_verify_alphabet_check(
                    &codeword,
                    rl,
                    &query_indices,
                    &packed_interleaved_alphabets,
                ),
            }
        })
}
