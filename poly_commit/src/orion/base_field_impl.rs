use std::{iter, ops::Mul};

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

pub fn orion_commit_base_field<ComPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<ComPackF::Scalar>,
    scratch_pad: &mut OrionScratchPad<ComPackF>,
) -> OrionResult<OrionCommitment>
where
    ComPackF: SimdField,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<ComPackF::Scalar>(poly.num_vars());
    let packed_rows = row_num / ComPackF::PACK_SIZE;
    assert_eq!(row_num % ComPackF::PACK_SIZE, 0);

    assert_eq!(poly.hypercube_size() % ComPackF::PACK_SIZE, 0);
    let packed_evals = pack_from_base::<ComPackF>(poly.hypercube_basis_ref());

    commit_encoded(pk, &packed_evals, scratch_pad, packed_rows, msg_size)
}

pub fn orion_open_base_field<SimdEvalF, ComPackF, OpenPackF>(
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<ComPackF::Scalar>,
    point: &[SimdEvalF::Scalar],
    transcript: &mut impl Transcript,
    scratch_pad: &OrionScratchPad<ComPackF>,
) -> (SimdEvalF::Scalar, OrionProof<SimdEvalF::Scalar>)
where
    SimdEvalF: SimdField + ExtensionField,
    SimdEvalF::BaseField: SimdField + Mul<OpenPackF, Output = SimdEvalF::BaseField>,
    SimdEvalF::Scalar: ExtensionField<BaseField = <SimdEvalF::BaseField as SimdField>::Scalar>,
    ComPackF: SimdField,
    OpenPackF: SimdField<Scalar = ComPackF::Scalar>,
{
    let (_, msg_size) = OrionSRS::evals_shape::<ComPackF::Scalar>(poly.num_vars());

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: pack evaluations for linear combinations in evaulation/proximity tests
    assert_eq!(poly.hypercube_size() % OpenPackF::PACK_SIZE, 0);
    let packed_evals = pack_from_base(poly.hypercube_basis_ref());

    // NOTE: pre-compute the eq linear combine coeffs for linear combination
    let eq_col_coeffs = {
        let mut eq_vars = point[..num_vars_in_com_simd].to_vec();
        eq_vars.extend_from_slice(&point[num_vars_in_com_simd + num_vars_in_msg..]);
        EqPolynomial::build_eq_x_r(&eq_vars)
    };

    // NOTE: pre-declare the spaces for returning evaluation and proximity queries
    let mut eval_row = vec![SimdEvalF::Scalar::ZERO; msg_size];

    let proximity_test_num = pk.proximity_repetitions::<SimdEvalF::Scalar>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![SimdEvalF::Scalar::ZERO; msg_size]; proximity_test_num];

    match <<SimdEvalF as ExtensionField>::BaseField as SimdField>::Scalar::NAME {
        GF2::NAME => lut_open_linear_combine(
            ComPackF::PACK_SIZE,
            &packed_evals,
            &eq_col_coeffs,
            &mut eval_row,
            &mut proximity_rows,
            transcript,
        ),
        _ => simd_open_linear_combine::<OpenPackF, SimdEvalF, _>(
            ComPackF::PACK_SIZE,
            &packed_evals,
            &eq_col_coeffs,
            &mut eval_row,
            &mut proximity_rows,
            transcript,
        ),
    }
    drop(packed_evals);

    // NOTE: working on evaluation on top of evaluation response
    let mut scratch = vec![SimdEvalF::Scalar::ZERO; msg_size];
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

pub fn orion_verify_base_field<SimdEvalF, ComPackF, OpenPackF>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &[SimdEvalF::Scalar],
    evaluation: SimdEvalF::Scalar,
    transcript: &mut impl Transcript,
    proof: &OrionProof<SimdEvalF::Scalar>,
) -> bool
where
    SimdEvalF: SimdField + ExtensionField,
    SimdEvalF::BaseField: SimdField + Mul<OpenPackF, Output = SimdEvalF::BaseField>,
    SimdEvalF::Scalar: ExtensionField<BaseField = <SimdEvalF::BaseField as SimdField>::Scalar>,
    ComPackF: SimdField,
    OpenPackF: SimdField<Scalar = ComPackF::Scalar>,
{
    let (row_num, msg_size) = OrionSRS::evals_shape::<ComPackF::Scalar>(point.len());

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: working on evaluation response, evaluate the rest of the response
    let mut scratch = vec![SimdEvalF::Scalar::ZERO; msg_size];
    let final_eval = RefMultiLinearPoly::from_ref(&proof.eval_row).evaluate_with_buffer(
        &point[num_vars_in_com_simd..num_vars_in_com_simd + num_vars_in_msg],
        &mut scratch,
    );

    if final_eval != evaluation {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<SimdEvalF::Scalar>(PCS_SOUNDNESS_BITS);
    let random_linear_combinations: Vec<Vec<SimdEvalF::Scalar>> = (0..proximity_reps)
        .map(|_| transcript.generate_field_elements::<SimdEvalF::Scalar>(row_num))
        .collect();
    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_usize_vector(query_num);

    // NOTE: check consistency in MT in the opening trees and against the commitment tree
    if !orion_mt_verify(vk, &query_indices, &proof.query_openings, commitment) {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but pack them back into look up table acceptable formats
    let packed_interleaved_alphabets: Vec<_> = proof
        .query_openings
        .iter()
        .map(|p| -> Vec<_> {
            p.unpack_field_elems::<ComPackF>()
                .chunks(OpenPackF::PACK_SIZE)
                .map(OpenPackF::pack)
                .collect()
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

            match <<SimdEvalF as ExtensionField>::BaseField as SimdField>::Scalar::NAME {
                GF2::NAME => lut_verify_alphabet_check(
                    &codeword,
                    rl,
                    &query_indices,
                    &packed_interleaved_alphabets,
                ),
                _ => simd_verify_alphabet_check::<OpenPackF, SimdEvalF>(
                    &codeword,
                    rl,
                    &query_indices,
                    &packed_interleaved_alphabets,
                ),
            }
        })
}
