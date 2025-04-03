use std::{
    io::{Cursor, Read},
    vec,
};

use arith::Field;
use circuit::{Circuit, CircuitLayer};
use config::GKRScheme;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use rayon::prelude::*;
use serdes::ExpSerde;
use sumcheck::{VerifierScratchPad, SUMCHECK_GKR_DEGREE, SUMCHECK_GKR_SIMD_MPI_DEGREE};
use transcript::Transcript;
use utils::timer::Timer;

use super::common::sumcheck_verify_gkr_layer;
use crate::prover::gkr_par_verifier::SumcheckLayerState;

pub fn parse_proof<'a, C: GKRFieldConfig>(
    circuit: &'a Circuit<C>,
    mut proof_reader: impl Read,
    mpi_config: &MPIConfig,
) -> Vec<(&'a CircuitLayer<C>, Vec<u8>, SumcheckLayerState<C>)> {
    circuit
        .layers
        .iter()
        .rev()
        .map(|layer| {
            let sumcheck_layer_state =
                SumcheckLayerState::<C>::deserialize_from(&mut proof_reader).unwrap();

            let proof_size_n_challenge_fields = layer.input_var_num * (!layer.structure_info.skip_sumcheck_phase_two as usize + 1) * (SUMCHECK_GKR_DEGREE + 1) // polynomials for variable x, y
                + (!layer.structure_info.skip_sumcheck_phase_two as usize + 1) // vx_claim, vy_claim
                + (C::get_field_pack_size().ilog2() as usize + mpi_config.world_size().ilog2() as usize) * (SUMCHECK_GKR_SIMD_MPI_DEGREE + 1); // polynomials for variable simd, mpi
            let proof_size_n_bytes = proof_size_n_challenge_fields * C::ChallengeField::SIZE;
            let mut proof_bytes = vec![0u8; proof_size_n_bytes];
            proof_reader.read_exact(&mut proof_bytes).unwrap();

            (layer, proof_bytes, sumcheck_layer_state)
        })
        .collect()
}

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_par_verifier_verify<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    mpi_config: &MPIConfig,
    circuit: &Circuit<C>,
    public_input: &[C::SimdCircuitField],
    claimed_v: &C::ChallengeField,
    transcript: &mut T,
    mut proof_reader: impl Read,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Option<Vec<C::ChallengeField>>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::ChallengeField,
    Option<C::ChallengeField>,
) {
    let timer = Timer::new("gkr_par_verifier_verify", true);
    let sp = VerifierScratchPad::<C>::new(circuit, mpi_config.world_size());

    let mut rz0 = vec![];
    let mut r_simd = vec![];
    let mut r_mpi = vec![];

    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.generate_challenge_field_element());
    }

    for _ in 0..C::get_field_pack_size().trailing_zeros() {
        r_simd.push(transcript.generate_challenge_field_element());
    }

    for _ in 0..mpi_config.world_size().trailing_zeros() {
        r_mpi.push(transcript.generate_challenge_field_element());
    }

    let transcript_state = transcript.hash_and_return_state();
    let verification_exec_units = parse_proof(circuit, &mut proof_reader, mpi_config);

    let init_state = &verification_exec_units.first().unwrap().2;
    let mut verified = init_state.transcript_state == transcript_state
        && init_state.rz0 == rz0
        && init_state.rz1.is_none()
        && init_state.r_simd == r_simd
        && init_state.r_mpi == r_mpi
        && init_state.alpha.is_none()
        && init_state.claimed_v0 == *claimed_v
        && init_state.claimed_v1.is_none();

    let world_size = mpi_config.world_size();
    let sumcheck_finished_states = verification_exec_units
        .par_iter()
        .enumerate()
        .map(|(i, (layer, proof_bytes, state))| {
            let mut sp = sp.clone();
            let mut local_transcript = T::new();
            local_transcript.set_state(&state.transcript_state);

            let mut cursor = Cursor::new(proof_bytes);
            let (verified, rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1) =
                sumcheck_verify_gkr_layer(
                    GKRScheme::GKRParVerifier,
                    &MPIConfig::new_for_verifier(world_size as i32),
                    layer,
                    public_input,
                    &state.rz0,
                    &state.rz1,
                    &state.r_simd,
                    &state.r_mpi,
                    state.claimed_v0,
                    state.claimed_v1,
                    state.alpha,
                    &mut cursor,
                    &mut local_transcript,
                    &mut sp,
                    i == 0,
                );

            let alpha = if rz1.is_some() {
                Some(local_transcript.generate_challenge_field_element())
            } else {
                None
            };

            (
                verified,
                SumcheckLayerState::<C> {
                    transcript_state: local_transcript.hash_and_return_state(),
                    rz0: rz0.to_vec(),
                    rz1: rz1.clone(),
                    r_simd: r_simd.to_vec(),
                    r_mpi: r_mpi.to_vec(),
                    alpha,
                    claimed_v0,
                    claimed_v1,
                },
            )
        })
        .collect::<Vec<_>>();

    verified &= verification_exec_units
        .iter()
        .skip(1)
        .zip(
            sumcheck_finished_states
                .iter()
                .take(sumcheck_finished_states.len() - 1),
        )
        .all(|((_, _, end_state), (verified, begin_state_next))| {
            *verified && (end_state == begin_state_next)
        });

    timer.stop();

    let final_state = sumcheck_finished_states.last().unwrap();
    transcript.set_state(&final_state.1.transcript_state);

    (
        final_state.0 && verified,
        final_state.1.rz0.to_vec(),
        final_state.1.rz1.clone(),
        final_state.1.r_simd.to_vec(),
        final_state.1.r_mpi.to_vec(),
        final_state.1.claimed_v0,
        final_state.1.claimed_v1,
    )
}
