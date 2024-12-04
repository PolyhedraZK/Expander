use std::iter;

use arith::{Field, SimdField};
use gkr_field_config::GKRFieldConfig;
use itertools::izip;
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;

use crate::{
    orion::utils::*, traits::TensorCodeIOPPCS, ExpanderGKRChallenge, OrionCommitment, OrionProof,
    OrionSRS, PCS_SOUNDNESS_BITS,
};

// NOTE: We assume this API is only used under PCS integration for expander GKR,
// and this method represents the local work for PCS opening in a party in MPI.
pub(crate) fn orion_open_simd_field_mpi<C, ComPackF, OpenPackF, T>(
    mpi_world_size: usize,
    mpi_rank: usize,
    pk: &OrionSRS,
    poly: &MultiLinearPoly<C::SimdCircuitField>,
    eval_point: &ExpanderGKRChallenge<C>,
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<C::CircuitField, ComPackF>,
) -> OrionProof<C::ChallengeField>
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    OpenPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    let local_point = eval_point.local_xs();
    assert_eq!(eval_point.x_mpi.len(), mpi_world_size.ilog2() as usize);

    let (row_num, msg_size) = {
        let num_vars = poly.get_num_vars() + C::SimdCircuitField::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars, local_point.len());

        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<C::CircuitField>(num_vars);
        let row_num = row_field_elems / C::SimdCircuitField::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_row = row_num.ilog2() as usize;
    let num_vars_in_unpacked_msg = local_point.len() - num_vars_in_row;

    // NOTE: transpose and shuffle evaluations (repack evaluations in another direction)
    // for linear combinations in evaulation/proximity tests
    let mut evals = poly.coeffs.clone();
    let packed_shuffled_evals: Vec<OpenPackF> = transpose_and_shuffle_simd(&mut evals, row_num);
    drop(evals);

    // NOTE: declare the look up tables for column sums
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<C::ChallengeField>::new(OpenPackF::PACK_SIZE, tables_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    // NOTE: working on evaluation response of tensor code IOP based PCS
    // The difference is that, the LUTs multiply with MPI eq coeff weight
    let mut eval_row = vec![C::ChallengeField::ZERO; msg_size * C::SimdCircuitField::PACK_SIZE];

    let eq_coeffs: Vec<_> = {
        let mpi_eq_coeffs = EqPolynomial::build_eq_x_r(&eval_point.x_mpi);
        let mpi_eq_weight = mpi_eq_coeffs[mpi_rank];
        let eqs = EqPolynomial::build_eq_x_r(&local_point[num_vars_in_unpacked_msg..]);
        eqs.iter().map(|t| *t * mpi_eq_weight).collect()
    };
    luts.build(&eq_coeffs);

    izip!(packed_shuffled_evals.chunks(tables_num), &mut eval_row)
        .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));

    // NOTE: draw random linear combination out
    // and compose proximity response(s) of tensor code IOP based PCS
    let proximity_reps = pk.proximity_repetitions::<C::ChallengeField>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows =
        vec![
            vec![C::ChallengeField::ZERO; msg_size * C::SimdCircuitField::PACK_SIZE];
            proximity_reps
        ];

    proximity_rows.iter_mut().for_each(|row_buffer| {
        let random_coeffs: Vec<_> = {
            let mpi_weights = transcript.generate_challenge_field_elements(mpi_world_size);
            let mpi_weight = mpi_weights[mpi_rank];
            let random_coeffs = transcript.generate_challenge_field_elements(row_num);
            random_coeffs.iter().map(|t| *t * mpi_weight).collect()
        };
        luts.build(&random_coeffs);

        izip!(packed_shuffled_evals.chunks(tables_num), row_buffer)
            .for_each(|(p_col, res)| *res = luts.lookup_and_sum(p_col));
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn orion_verify_simd_field_mpi<C, ComPackF, OpenPackF, T>(
    mpi_world_size: usize,
    mpi_rank: usize,
    vk: &OrionSRS,
    commitment: &OrionCommitment,
    point: &ExpanderGKRChallenge<C>,
    evaluation: C::ChallengeField,
    transcript: &mut T,
    proof: &OrionProof<C::ChallengeField>,
) -> bool
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    OpenPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    let local_xs = point.local_xs();

    let (row_num, msg_size) = {
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<C::CircuitField>(local_xs.len());
        let row_num = row_field_elems / C::SimdCircuitField::PACK_SIZE;
        (row_num, msg_size)
    };

    let num_vars_in_row = row_num.ilog2() as usize;
    let num_vars_in_unpacked_msg = local_xs.len() - num_vars_in_row;

    // NOTE: working on evaluation response, evaluate the rest of the response
    let mut scratch = vec![C::ChallengeField::ZERO; msg_size * C::SimdCircuitField::PACK_SIZE];
    let final_eval = MultiLinearPoly::evaluate_with_buffer(
        &proof.eval_row,
        &local_xs[..num_vars_in_unpacked_msg],
        &mut scratch,
    );

    let mpi_eq_coeffs = EqPolynomial::build_eq_x_r(&point.x_mpi);
    let mpi_eq_weight = mpi_eq_coeffs[mpi_rank];
    if final_eval != evaluation * mpi_eq_weight {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<C::ChallengeField>(PCS_SOUNDNESS_BITS);
    let random_linear_combinations: Vec<Vec<C::ChallengeField>> = (0..proximity_reps)
        .map(|_| {
            let mpi_weights = transcript.generate_challenge_field_elements(mpi_world_size);
            let mpi_weight = mpi_weights[mpi_rank];
            let random_coeffs = transcript.generate_challenge_field_elements(row_num);
            random_coeffs.iter().map(|t| *t * mpi_weight).collect()
        })
        .collect();
    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);

    // NOTE: check consistency in MT in the opening trees and against the commitment tree
    if !orion_mt_verify(vk, &query_indices, &proof.query_openings, commitment) {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but reshuffle the packed elements into another direction
    let mut scratch = vec![C::CircuitField::ZERO; C::SimdCircuitField::PACK_SIZE * row_num];
    let shuffled_interleaved_alphabet: Vec<Vec<OpenPackF>> = proof
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

    // NOTE: declare the look up tables for column sums
    let tables_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<C::ChallengeField>::new(OpenPackF::PACK_SIZE, tables_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    let eq_linear_combination = {
        let mpi_eq_coeffs = EqPolynomial::build_eq_x_r(&point.x_mpi);
        let mpi_eq_weight = mpi_eq_coeffs[mpi_rank];
        let eqs = EqPolynomial::build_eq_x_r(&local_xs[num_vars_in_unpacked_msg..]);
        eqs.iter().map(|t| *t * mpi_eq_weight).collect()
    };

    let mut scratch_msg = vec![C::ChallengeField::ZERO; C::SimdCircuitField::PACK_SIZE * msg_size];
    let mut scratch_codeword =
        vec![C::ChallengeField::ZERO; C::SimdCircuitField::PACK_SIZE * vk.codeword_len()];

    izip!(&random_linear_combinations, &proof.proximity_rows)
        .chain(iter::once((&eq_linear_combination, &proof.eval_row)))
        .all(|(rl, msg)| {
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

            luts.build(rl);

            izip!(&query_indices, &shuffled_interleaved_alphabet).all(
                |(&qi, interleaved_alphabet)| {
                    let index = qi % vk.codeword_len();

                    let simd_starts = index * C::SimdCircuitField::PACK_SIZE;
                    let simd_ends = (index + 1) * C::SimdCircuitField::PACK_SIZE;

                    izip!(
                        &codeword[simd_starts..simd_ends],
                        interleaved_alphabet.chunks(tables_num)
                    )
                    .all(|(expected_alphabet, packed_index)| {
                        let alphabet = luts.lookup_and_sum(packed_index);
                        alphabet == *expected_alphabet
                    })
                },
            )
        })
}

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

    let eq_local_coeffs = EqPolynomial::build_eq_x_r(&local_xs[num_vars_in_unpacked_msg..]);
    let eq_worlds_coeffs = EqPolynomial::build_eq_x_r(&eval_point.x_mpi);

    // NOTE: working on evaluation response
    let mut scratch =
        vec![C::ChallengeField::ZERO; mpi_world_size * C::SimdCircuitField::PACK_SIZE * msg_size];
    let final_eval = MultiLinearPoly::evaluate_with_buffer(
        &proof.eval_row,
        &local_xs[..num_vars_in_unpacked_msg],
        &mut scratch[..C::SimdCircuitField::PACK_SIZE * msg_size],
    );
    if final_eval != eval {
        return false;
    }

    // NOTE: working on proximity responses, draw random linear combinations
    // then draw query points from fiat shamir transcripts
    let proximity_reps = vk.proximity_repetitions::<C::ChallengeField>(PCS_SOUNDNESS_BITS);
    let (proximity_worlds_coeffs, proximity_local_coeffs): (Vec<_>, Vec<_>) = (0..proximity_reps)
        .map(|_| {
            (
                transcript.generate_challenge_field_elements(mpi_world_size),
                transcript.generate_challenge_field_elements(row_num),
            )
        })
        .unzip();

    let query_num = vk.query_complexity(PCS_SOUNDNESS_BITS);
    let query_indices = transcript.generate_challenge_index_vector(query_num);

    // NOTE: check all merkle paths, and check merkle roots against commitment
    let (mt_verifications, roots): (Vec<_>, Vec<_>) = proof
        .query_openings
        .chunks(query_num)
        .map(|queries| {
            let root = queries[0].root();
            (orion_mt_verify(vk, &query_indices, queries, &root), root)
        })
        .unzip();

    if !itertools::all(&mt_verifications, |v| *v) {
        return false;
    }

    let final_tree_height = 1 + roots.len().ilog2();
    let (internals, _) = tree::Tree::new_with_leaf_nodes(roots, final_tree_height);
    if internals[0] != *commitment {
        return false;
    }

    // NOTE: prepare the interleaved alphabets from the MT paths,
    // but reshuffle the packed elements into another direction
    let mut scratch_f = vec![C::CircuitField::ZERO; C::SimdCircuitField::PACK_SIZE * row_num];
    let shuffled_interleaved_alphabet: Vec<_> = proof
        .query_openings
        .iter()
        .map(|c| -> Vec<_> {
            let mut elts = c.unpack_field_elems::<C::CircuitField, ComPackF>();
            transpose_in_place(&mut elts, &mut scratch_f, row_num);
            elts.chunks(OpenPackF::PACK_SIZE)
                .map(OpenPackF::pack)
                .collect()
        })
        .collect();

    // NOTE: decide if expected alphabet matches actual responses
    let table_num = row_num / OpenPackF::PACK_SIZE;
    let mut luts = SubsetSumLUTs::<C::ChallengeField>::new(OpenPackF::PACK_SIZE, table_num);
    assert_eq!(row_num % OpenPackF::PACK_SIZE, 0);

    let mut scratch_q =
        vec![C::ChallengeField::ZERO; mpi_world_size * C::SimdCircuitField::PACK_SIZE * query_num];
    let mut codeword =
        vec![C::ChallengeField::ZERO; C::SimdCircuitField::PACK_SIZE * vk.codeword_len()];

    izip!(
        &proximity_local_coeffs,
        &proximity_worlds_coeffs,
        &proof.proximity_rows
    )
    .chain(iter::once((
        &eq_local_coeffs,
        &eq_worlds_coeffs,
        &proof.eval_row,
    )))
    .all(|(local_coeffs, worlds_coeffs, msg)| {
        // NOTE: compute final actual alphabets cross worlds
        luts.build(local_coeffs);
        let mut each_world_alphabets: Vec<_> = shuffled_interleaved_alphabet
            .iter()
            .flat_map(|c| -> Vec<_> {
                c.chunks(table_num)
                    .map(|ts| luts.lookup_and_sum(ts))
                    .collect()
            })
            .collect();
        transpose_in_place(&mut each_world_alphabets, &mut scratch_q, mpi_world_size);
        let actual_alphabets: Vec<_> = each_world_alphabets
            .chunks(mpi_world_size)
            .map(|rs| izip!(rs, worlds_coeffs).map(|(&l, &r)| l * r).sum())
            .collect();

        // NOTE: compute SIMD codewords from the message
        let mut msg_cloned = msg.clone();
        transpose_in_place(
            &mut msg_cloned,
            &mut scratch[..C::SimdCircuitField::PACK_SIZE * msg_size],
            msg_size,
        );
        izip!(
            msg_cloned.chunks(msg_size),
            codeword.chunks_mut(vk.codeword_len())
        )
        .for_each(|(msg, c)| vk.code_instance.encode_in_place(msg, c).unwrap());
        transpose_in_place(
            &mut codeword,
            &mut scratch[..C::SimdCircuitField::PACK_SIZE * vk.codeword_len()],
            C::SimdCircuitField::PACK_SIZE,
        );

        // NOTE: check actual SIMD alphabets against expected SIMD alphabets
        izip!(
            &query_indices,
            actual_alphabets.chunks(C::SimdCircuitField::PACK_SIZE)
        )
        .all(|(qi, actual_alphabets)| {
            let index = qi % vk.codeword_len();

            let simd_starts = index * C::SimdCircuitField::PACK_SIZE;
            let simd_ends = (index + 1) * C::SimdCircuitField::PACK_SIZE;

            izip!(&codeword[simd_starts..simd_ends], actual_alphabets).all(|(ec, ac)| ec == ac)
        })
    })
}
