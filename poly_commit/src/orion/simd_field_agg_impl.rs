use std::{iter, ops::Mul};

use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use gkr_engine::{ExpanderSingleVarChallenge, Transcript};
use itertools::izip;
use polynomials::{EqPolynomial, MultilinearExtension, RefMultiLinearPoly};

use crate::{
    orion::utils::*, traits::TensorCodeIOPPCS, OrionCommitment, OrionProof, OrionSRS,
    PCS_SOUNDNESS_BITS,
};

pub(crate) fn orion_verify_simd_field_aggregated<SimdPolyF, SimdEvalF, ComPackF>(
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    eval_point: &ExpanderSingleVarChallenge<SimdEvalF::Scalar>,
    eval: SimdEvalF::Scalar,
    transcript: &mut impl Transcript,
    proof: &OrionProof<SimdEvalF::Scalar>,
) -> bool
where
    SimdPolyF: SimdField,
    SimdEvalF: SimdField + ExtensionField,
    SimdEvalF::BaseField: SimdField + Mul<SimdPolyF, Output = SimdEvalF::BaseField>,
    SimdEvalF::Scalar: ExtensionField<BaseField = <SimdEvalF::BaseField as SimdField>::Scalar>,
    ComPackF: SimdField<Scalar = SimdPolyF::Scalar>,
{
    let mpi_world_size = 1 << eval_point.r_mpi.len();
    let local_num_vars = eval_point.num_vars() - eval_point.r_mpi.len();
    assert_eq!(local_num_vars, vk.num_vars);

    let (row_num, msg_size) = {
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<SimdPolyF::Scalar>(local_num_vars);
        let row_num = row_field_elems / SimdPolyF::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    let global_xs = eval_point.global_xs();

    // NOTE: working on evaluation response
    let mut scratch = vec![SimdEvalF::Scalar::ZERO; msg_size];
    let final_eval = RefMultiLinearPoly::from_ref(&proof.eval_row).evaluate_with_buffer(
        &global_xs[num_vars_in_com_simd..num_vars_in_com_simd + num_vars_in_msg],
        &mut scratch,
    );
    if final_eval != eval {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<SimdEvalF::Scalar>(PCS_SOUNDNESS_BITS);
    let proximity_local_coeffs: Vec<Vec<SimdEvalF::Scalar>> = (0..proximity_reps)
        .map(|_| {
            transcript.generate_field_elements::<SimdEvalF::Scalar>(row_num * SimdEvalF::PACK_SIZE)
        })
        .collect();

    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_usize_vector(query_num);

    let proximity_worlds_coeffs: Vec<Vec<SimdEvalF::Scalar>> = (0..proximity_reps)
        .map(|_| transcript.generate_field_elements::<SimdEvalF::Scalar>(mpi_world_size))
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
        let internals = tree::Tree::new_with_leaf_nodes(&roots, final_tree_height);
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

    let mut packed_interleaved_alphabets: Vec<Vec<SimdPolyF>> =
        vec![Vec::new(); query_num];

    let concatenated_packed_interleaved_alphabets: Vec<_> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            let elts = c.unpack_field_elems::<ComPackF>();
            elts.chunks(SimdPolyF::PACK_SIZE)
                .map(SimdPolyF::pack)
                .collect()
        })
        .collect();

    concatenated_packed_interleaved_alphabets
        .chunks(query_num)
        .for_each(|alphabets| {
            izip!(&mut packed_interleaved_alphabets, alphabets)
                .for_each(|(packed, alphabet)| packed.extend_from_slice(alphabet))
        });

    let mut eq_vars = vec![SimdEvalF::Scalar::ZERO; eval_point.num_vars() - num_vars_in_msg];
    eq_vars[..num_vars_in_com_simd].copy_from_slice(&global_xs[..num_vars_in_com_simd]);
    eq_vars[num_vars_in_com_simd..]
        .copy_from_slice(&global_xs[num_vars_in_com_simd + num_vars_in_msg..]);

    let eq_col_coeffs = EqPolynomial::build_eq_x_r(&eq_vars);

    let proximity_coeffs: Vec<Vec<SimdEvalF::Scalar>> = (0..proximity_reps)
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

            match <<SimdEvalF as ExtensionField>::BaseField as SimdField>::Scalar::NAME {
                GF2::NAME => lut_verify_alphabet_check(
                    &codeword,
                    rl,
                    &query_indices,
                    &packed_interleaved_alphabets,
                ),
                _ => simd_verify_alphabet_check::<SimdPolyF, SimdEvalF>(
                    &codeword,
                    rl,
                    &query_indices,
                    &packed_interleaved_alphabets,
                ),
            }
        })
}
