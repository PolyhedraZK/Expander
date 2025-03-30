use std::iter;

use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use itertools::{chain, izip};
use polynomials::{EqPolynomial, MultilinearExtension, RefMultiLinearPoly};
use transcript::Transcript;
use tree::LEAF_BYTES;

use crate::{
    orion::{
        utils::{lut_verify_alphabet_check, orion_mt_verify, simd_verify_alphabet_check},
        OrionCommitment, OrionProof, OrionSRS,
    },
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

#[inline(always)]
pub fn orion_verify<F, SimdF, EvalF, ComPackF, T>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &[EvalF],
    mpi_point: &[EvalF],
    evaluation: EvalF,
    transcript: &mut T,
    proof: &OrionProof<EvalF>,
) -> bool
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let (_, msg_size) = OrionSRS::evals_shape::<F>(point.len());

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
    let random_linear_combinations: Vec<_> = (0..proximity_reps)
        .map(|_| {
            let num_vars = point.len() - num_vars_in_msg + mpi_point.len();
            let rand = transcript.generate_challenge_field_elements(num_vars);
            EqPolynomial::build_eq_x_r(&rand)
        })
        .collect();

    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);

    // NOTE: check consistency in MT in the opening trees and against the commitment tree
    let world_size = 1 << mpi_point.len();
    if !orion_mt_verify(
        vk,
        world_size,
        &query_indices,
        &proof.query_openings,
        commitment,
    ) {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but reshuffle the packed elements into another direction
    let packed_interleaved_alphabets: Vec<Vec<SimdF>> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            let len = c.leaves.len() * LEAF_BYTES / SimdF::SIZE;
            let cap = c.leaves.capacity() * LEAF_BYTES / SimdF::SIZE;

            let leaves_cast =
                unsafe { Vec::from_raw_parts(c.leaves.as_ptr() as *mut SimdF, len, cap) };

            let res = leaves_cast.clone();
            leaves_cast.leak();

            res
        })
        .collect();

    let eq_col_coeffs = {
        let mut eq_vars = point[..num_vars_in_com_simd].to_vec();
        eq_vars.extend_from_slice(&point[num_vars_in_com_simd + num_vars_in_msg..]);
        eq_vars.extend_from_slice(mpi_point);
        EqPolynomial::build_eq_x_r(&eq_vars)
    };

    chain!(
        izip!(&random_linear_combinations, &proof.proximity_rows),
        iter::once((&eq_col_coeffs, &proof.eval_row))
    )
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
