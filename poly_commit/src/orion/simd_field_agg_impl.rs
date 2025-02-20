use std::iter;

use arith::{Field, SimdField};
use gf2::GF2;
use gkr_field_config::GKRFieldConfig;
use itertools::izip;
use polynomials::{EqPolynomial, MultilinearExtension, RefMultiLinearPoly};
use transcript::Transcript;

use crate::{
    orion::utils::*, traits::TensorCodeIOPPCS, ExpanderGKRChallenge, OrionCommitment, OrionProof,
    OrionSRS, PCS_SOUNDNESS_BITS,
};

pub(crate) fn orion_verify_simd_field_aggregated<C, ComPackF, T>(
    mpi_world_size: usize,
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    eval_point: &ExpanderGKRChallenge<C>,
    eval: C::ChallengeField,
    transcript: &mut T,
    proof: &OrionProof<C::ChallengeField>,
) -> bool
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    let local_num_vars = eval_point.num_vars() - mpi_world_size.ilog2() as usize;
    assert_eq!(local_num_vars, vk.num_vars);

    let (row_num, msg_size) = {
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<C::CircuitField>(local_num_vars);
        let row_num = row_field_elems / C::SimdCircuitField::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_simd = C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    let global_xs = eval_point.global_xs();

    // NOTE: working on evaluation response
    let mut scratch = vec![C::ChallengeField::ZERO; msg_size];
    let final_eval = RefMultiLinearPoly::from_ref(&proof.eval_row).evaluate_with_buffer(
        &global_xs[num_vars_in_simd..num_vars_in_simd + num_vars_in_msg],
        &mut scratch,
    );
    if final_eval != eval {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<C::ChallengeField>(PCS_SOUNDNESS_BITS);
    let proximity_local_coeffs: Vec<Vec<C::ChallengeField>> = (0..proximity_reps)
        .map(|_| {
            transcript.generate_challenge_field_elements(row_num * C::SimdCircuitField::PACK_SIZE)
        })
        .collect();

    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);

    let proximity_worlds_coeffs: Vec<Vec<C::ChallengeField>> = (0..proximity_reps)
        .map(|_| transcript.generate_challenge_field_elements(mpi_world_size))
        .collect();

    // NOTE: work on the Merkle tree path validity
    let roots: Vec<_> = proof
        .query_openings
        .chunks(query_num)
        .map(|qs| qs[0].root())
        .collect();

    let final_root = {
        // NOTE: check all merkle paths, and check merkle roots against commitment
        let final_tree_height = 1 + roots.len().ilog2();
        let (internals, _) = tree::Tree::new_with_leaf_nodes(roots.clone(), final_tree_height);
        internals[0]
    };
    if final_root != *commitment {
        return false;
    }

    if izip!(proof.query_openings.chunks(query_num), &roots)
        .any(|(paths, r)| !orion_mt_verify(vk, &query_indices, paths, r))
    {
        return false;
    }

    let mut packed_interleaved_alphabets: Vec<Vec<C::SimdCircuitField>> =
        vec![Vec::new(); query_num];

    let concatenated_packed_interleaved_alphabets: Vec<_> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            let elts = c.unpack_field_elems::<C::CircuitField, ComPackF>();
            elts.chunks(C::SimdCircuitField::PACK_SIZE)
                .map(C::SimdCircuitField::pack)
                .collect()
        })
        .collect();

    concatenated_packed_interleaved_alphabets
        .chunks(query_num)
        .for_each(|alphabets| {
            izip!(&mut packed_interleaved_alphabets, alphabets)
                .for_each(|(packed, alphabet)| packed.extend_from_slice(alphabet))
        });

    let mut eq_vars = vec![C::ChallengeField::ZERO; eval_point.num_vars() - num_vars_in_msg];
    eq_vars[..num_vars_in_simd].copy_from_slice(&global_xs[..num_vars_in_simd]);
    eq_vars[num_vars_in_simd..].copy_from_slice(&global_xs[num_vars_in_simd + num_vars_in_msg..]);

    let eq_col_coeffs = EqPolynomial::build_eq_x_r(&eq_vars);

    let proximity_coeffs: Vec<Vec<C::ChallengeField>> = (0..proximity_reps)
        .map(|i| {
            proximity_worlds_coeffs[i]
                .iter()
                .flat_map(|w| {
                    proximity_local_coeffs[i]
                        .iter()
                        .map(|l| *l * *w)
                        .collect::<Vec<_>>()
                })
                .collect()
        })
        .collect();

    // NOTE: decide if expected alphabet matches actual responses
    izip!(&proximity_coeffs, &proof.proximity_rows)
        .chain(iter::once((&eq_col_coeffs, &proof.eval_row)))
        .all(|(rl, msg)| {
            let codeword = match vk.code_instance.encode(msg) {
                Ok(c) => c,
                _ => return false,
            };

            match C::CircuitField::NAME {
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
