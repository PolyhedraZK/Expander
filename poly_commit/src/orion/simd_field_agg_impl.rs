use std::iter;

use arith::{Field, SimdField};
use gkr_field_config::GKRFieldConfig;
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{
    orion::utils::*, traits::TensorCodeIOPPCS, ExpanderGKRChallenge, OrionCommitment, OrionProof,
    OrionSRS, PCS_SOUNDNESS_BITS,
};

pub(crate) fn orion_verify_simd_field_aggregated<C, ComPackF, OpenPackF, T>(
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
    OpenPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    let local_num_vars = eval_point.num_vars() - mpi_world_size.ilog2() as usize;
    assert_eq!(local_num_vars, vk.num_vars);

    let (row_num, msg_size) = {
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<C::CircuitField>(local_num_vars);
        let row_num = row_field_elems / C::SimdCircuitField::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_local_rows = row_num.ilog2() as usize;
    let num_vars_in_unpacked_msg = local_num_vars - num_vars_in_local_rows;
    let local_xs = eval_point.local_xs();

    // NOTE: working on evaluation response
    let mut scratch = vec![C::ChallengeField::ZERO; msg_size * C::SimdCircuitField::PACK_SIZE];
    let final_eval = MultiLinearPoly::evaluate_with_buffer(
        &proof.eval_row,
        &local_xs[..num_vars_in_unpacked_msg],
        &mut scratch,
    );
    if final_eval != eval {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<C::ChallengeField>(PCS_SOUNDNESS_BITS);
    let proximity_coeffs: Vec<Vec<C::ChallengeField>> = (0..proximity_reps)
        .map(|_| transcript.generate_challenge_field_elements(row_num))
        .collect();
    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);

    // NOTE: check all merkle paths, and check merkle roots against commitment
    let (mt_verify_res, roots): (Vec<_>, Vec<_>) = proof
        .query_openings
        .chunks(query_num)
        .map(|queries| {
            let root = queries[0].root();
            (orion_mt_verify(vk, &query_indices, queries, &root), root)
        })
        .unzip();

    if !mt_verify_res.iter().all(|v| *v) {
        return false;
    }

    let final_tree_height = 1 + roots.len().ilog2();
    let (internals, _) = tree::Tree::new_with_leaf_nodes(roots, final_tree_height);
    if internals[0] != *commitment {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but reshuffle the packed elements into another direction
    let mut scratch = vec![C::CircuitField::ZERO; C::SimdCircuitField::PACK_SIZE * row_num];
    let shuffled_interleaved_alphabet: Vec<_> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            let mut elts = c.unpack_field_elems::<C::CircuitField, ComPackF>();
            transpose_in_place(&mut elts, &mut scratch, row_num);
            elts.chunks(OpenPackF::PACK_SIZE)
                .map(OpenPackF::pack)
                .collect()
        })
        .collect();

    // NOTE: compute alphabets from proximity/evaluation coeffs
    let table_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<C::ChallengeField>::new(OpenPackF::PACK_SIZE, table_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    let eq_local_coeffs = EqPolynomial::build_eq_x_r(&local_xs[num_vars_in_unpacked_msg..]);
    luts.build(&eq_local_coeffs);

    let mut scratch =
        vec![C::ChallengeField::ZERO; mpi_world_size * C::SimdCircuitField::PACK_SIZE * msg_size];

    let mut eval_qs: Vec<_> = shuffled_interleaved_alphabet
        .iter()
        .flat_map(|c| -> Vec<_> {
            c.chunks(table_num)
                .map(|ts| luts.lookup_and_sum(ts))
                .collect()
        })
        .collect();
    transpose_in_place(&mut eval_qs, &mut scratch, mpi_world_size);

    let proximity_qs: Vec<_> = proximity_coeffs
        .iter()
        .map(|ps| {
            luts.build(ps);
            let mut worlds_proximity_resps: Vec<_> = shuffled_interleaved_alphabet
                .iter()
                .flat_map(|c| -> Vec<_> {
                    c.chunks(table_num)
                        .map(|ts| luts.lookup_and_sum(ts))
                        .collect()
                })
                .collect();
            transpose_in_place(&mut worlds_proximity_resps, &mut scratch, mpi_world_size);
            worlds_proximity_resps
        })
        .collect();

    // NOTE: sum up each worlds responses with weights
    let eq_worlds_coeffs = EqPolynomial::build_eq_x_r(&eval_point.x_mpi);
    let actual_evals: Vec<C::ChallengeField> = eval_qs
        .chunks(mpi_world_size)
        .map(|rs| inner_prod(rs, &eq_worlds_coeffs))
        .collect();

    let actual_proximity_resps: Vec<Vec<C::ChallengeField>> = proximity_qs
        .iter()
        .map(|ps| {
            let weights = transcript.generate_challenge_field_elements(mpi_world_size);
            ps.chunks(mpi_world_size)
                .map(|rs| inner_prod(rs, &weights))
                .collect()
        })
        .collect();

    // NOTE: decide if expected alphabet matches actual responses
    let mut scratch_msg = vec![C::ChallengeField::ZERO; C::SimdCircuitField::PACK_SIZE * msg_size];
    let mut scratch_codeword =
        vec![C::ChallengeField::ZERO; C::SimdCircuitField::PACK_SIZE * vk.codeword_len()];
    actual_proximity_resps
        .iter()
        .zip(proof.proximity_rows.iter())
        .chain(iter::once((&actual_evals, &proof.eval_row)))
        .all(|(actual_alphabets, msg)| {
            let mut msg_cloned = msg.clone();
            transpose_in_place(&mut msg_cloned, &mut scratch_msg, msg_size);
            let mut codeword: Vec<_> = msg_cloned
                .chunks(msg_size)
                .flat_map(|m| vk.code_instance.encode(m).unwrap())
                .collect();
            transpose_in_place(
                &mut codeword,
                &mut scratch_codeword,
                C::SimdCircuitField::PACK_SIZE,
            );

            query_indices
                .iter()
                .zip(actual_alphabets.chunks(C::SimdCircuitField::PACK_SIZE))
                .all(|(qi, simd_alphabets)| {
                    let index = qi % vk.codeword_len();

                    let simd_starts = index * C::SimdCircuitField::PACK_SIZE;
                    let simd_ends = (index + 1) * C::SimdCircuitField::PACK_SIZE;

                    codeword[simd_starts..simd_ends]
                        .iter()
                        .zip(simd_alphabets.iter())
                        .all(|(ec, ac)| ec == ac)
                })
        })
}

#[inline]
fn inner_prod<F: Field>(ls: &[F], rs: &[F]) -> F {
    ls.iter().zip(rs.iter()).map(|(&l, &r)| r * l).sum()
}
