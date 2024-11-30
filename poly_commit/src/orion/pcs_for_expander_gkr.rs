use std::iter;

use arith::{Field, SimdField};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::{EqPolynomial, MultiLinearPoly};
use transcript::Transcript;
use utils::{orion_mt_verify, transpose_in_place};

use crate::{
    orion::*, traits::TensorCodeIOPPCS, ExpanderGKRChallenge, PCSForExpanderGKR,
    StructuredReferenceString, PCS_SOUNDNESS_BITS,
};

impl<C, ComPackF, OpenPackF, T> PCSForExpanderGKR<C, T>
    for OrionSIMDFieldPCS<
        C::CircuitField,
        C::SimdCircuitField,
        C::ChallengeField,
        ComPackF,
        OpenPackF,
        T,
    >
where
    C: GKRFieldConfig,
    ComPackF: SimdField<Scalar = C::CircuitField>,
    OpenPackF: SimdField<Scalar = C::CircuitField>,
    T: Transcript<C::ChallengeField>,
{
    const NAME: &'static str = "OrionSIMDPCSForExpanderGKR";

    type Params = usize;
    type ScratchPad = OrionScratchPad<C::CircuitField, ComPackF>;

    type Commitment = OrionCommitment;
    type Opening = OrionProof<C::ChallengeField>;
    type SRS = OrionSRS;

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        let num_vars_each_core = *params - mpi_config.world_size();
        OrionSRS::from_random::<C::CircuitField>(
            num_vars_each_core,
            ORION_CODE_PARAMETER_INSTANCE,
            rng,
        )
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_config: &MPIConfig) -> Self::ScratchPad {
        Self::ScratchPad::default()
    }

    fn commit(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let num_vars_each_core = *params - mpi_config.world_size();
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        let commitment = orion_commit_simd_field(proving_key, poly, scratch_pad).unwrap();
        if mpi_config.world_size == 1 {
            return commitment;
        }

        let local_buffer = vec![commitment];
        let mut buffer = vec![Self::Commitment::default(); mpi_config.world_size()];
        mpi_config.gather_vec(&local_buffer, &mut buffer);

        // NOTE: Hang also assume that, linear GKR will take over the commitment
        // and force sync transcript hash state of subordinate machines to be the same.
        if !mpi_config.is_root() {
            return commitment;
        }

        let final_tree_height = 1 + buffer.len().ilog2();
        let (internals, _) = tree::Tree::new_with_leaf_nodes(buffer.clone(), final_tree_height);
        internals[0]
    }

    fn open(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &MultiLinearPoly<C::SimdCircuitField>,
        eval_point: &ExpanderGKRChallenge<C>,
        transcript: &mut T, // add transcript here to allow interactive arguments
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Opening {
        let num_vars_each_core = *params - mpi_config.world_size();
        assert_eq!(num_vars_each_core, proving_key.num_vars);

        let local_xs = eval_point.local_xs();
        let local_opening = orion_open_simd_field::<
            C::CircuitField,
            C::SimdCircuitField,
            C::ChallengeField,
            ComPackF,
            OpenPackF,
            T,
        >(proving_key, poly, &local_xs, transcript, scratch_pad);
        if mpi_config.world_size == 1 {
            return local_opening;
        }

        // NOTE: eval row combine from MPI
        let mpi_eq_coeffs = EqPolynomial::build_eq_x_r(&eval_point.x_mpi);
        let eval_row = mpi_config.coef_combine_vec(&local_opening.eval_row, &mpi_eq_coeffs);

        // NOTE: sample MPI linear combination coeffs for proximity rows,
        // and proximity rows combine with MPI
        let proximity_rows = local_opening
            .proximity_rows
            .iter()
            .map(|row| {
                let weights = transcript.generate_challenge_field_elements(mpi_config.world_size());
                mpi_config.coef_combine_vec(row, &weights)
            })
            .collect();

        // NOTE: gather all merkle paths
        let mut query_openings = vec![
            tree::RangePath::default();
            mpi_config.world_size() * local_opening.query_openings.len()
        ];
        mpi_config.gather_vec(&local_opening.query_openings, &mut query_openings);

        if !mpi_config.is_root() {
            return local_opening;
        }

        // NOTE: we only care about the root machine's opening as final proof, Hang assume.
        OrionProof {
            eval_row,
            proximity_rows,
            query_openings,
        }
    }

    fn verify(
        params: &Self::Params,
        mpi_config: &MPIConfig,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        eval_point: &ExpanderGKRChallenge<C>,
        eval: C::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        assert_eq!(*params, eval_point.num_vars());

        if mpi_config.world_size == 1 {
            return orion_verify_simd_field::<
                C::CircuitField,
                C::SimdCircuitField,
                C::ChallengeField,
                ComPackF,
                OpenPackF,
                T,
            >(
                verifying_key,
                commitment,
                &eval_point.local_xs(),
                eval,
                transcript,
                opening,
            );
        }

        // NOTE: we now assume that the input opening is from the root machine,
        // as proofs from other machines are typically undefined
        orion_verify_simd_field_aggregated::<C, ComPackF, OpenPackF, T>(
            mpi_config.world_size(),
            verifying_key,
            commitment,
            eval_point,
            eval,
            transcript,
            opening,
        )
    }
}

fn orion_verify_simd_field_aggregated<C, ComPackF, OpenPackF, T>(
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

    // NOTE: check all merkle paths
    if !proof
        .query_openings
        .chunks(query_num)
        .all(|queries| orion_mt_verify(vk, &query_indices, queries, &queries[0].root()))
    {
        return false;
    }

    // NOTE: collect each merkle roots, build final root against commitment
    let roots: Vec<_> = proof
        .query_openings
        .chunks(query_num)
        .map(|p| p[0].root())
        .collect();
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

fn inner_prod<F: Field>(ls: &[F], rs: &[F]) -> F {
    ls.iter().zip(rs.iter()).map(|(&l, &r)| r * l).sum()
}
