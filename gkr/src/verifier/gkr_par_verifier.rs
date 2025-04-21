use std::{
    io::{Cursor, Read},
    vec,
};

use arith::Field;
use circuit::{Circuit, CircuitLayer};
use gkr_engine::{ExpanderDualVarChallenge, ExpanderSingleVarChallenge, FieldEngine, GKRScheme, Transcript};
use rayon::prelude::*;
use serdes::ExpSerde;
use sumcheck::{VerifierScratchPad, SUMCHECK_GKR_DEGREE, SUMCHECK_GKR_SIMD_MPI_DEGREE};
use utils::timer::Timer;

use super::common::sumcheck_verify_gkr_layer;
use crate::prover::gkr_par_verifier::SumcheckLayerState;

pub fn parse_proof<F: FieldEngine>(
    circuit: &Circuit<F>,
    mut proof_reader: impl Read,
    proving_time_mpi_size: usize,
) -> Vec<(&CircuitLayer<F>, Vec<u8>, SumcheckLayerState<F>)> {
    circuit
        .layers
        .iter()
        .rev()
        .map(|layer| {
            let sumcheck_layer_state =
                SumcheckLayerState::<F>::deserialize_from(&mut proof_reader).unwrap();

            let proof_size_n_challenge_fields = layer.input_var_num * (!layer.structure_info.skip_sumcheck_phase_two as usize + 1) * (SUMCHECK_GKR_DEGREE + 1) // polynomials for variable x, y
                + (!layer.structure_info.skip_sumcheck_phase_two as usize + 1) // vx_claim, vy_claim
                + (F::get_field_pack_size().ilog2() as usize + proving_time_mpi_size.ilog2() as usize) * (SUMCHECK_GKR_SIMD_MPI_DEGREE + 1); // polynomials for variable simd, mpi
            let proof_size_n_bytes = proof_size_n_challenge_fields * F::ChallengeField::SIZE;
            let mut proof_bytes = vec![0u8; proof_size_n_bytes];
            proof_reader.read_exact(&mut proof_bytes).unwrap();

            (layer, proof_bytes, sumcheck_layer_state)
        })
        .collect()
}

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_par_verifier_verify<F: FieldEngine, T: Transcript<F::ChallengeField>>(
    proving_time_mpi_size: usize,
    circuit: &Circuit<F>,
    public_input: &[F::SimdCircuitField],
    claimed_v: &F::ChallengeField,
    transcript: &mut T,
    mut proof_reader: impl Read,
) -> (
    bool,
    ExpanderDualVarChallenge<F>,
    F::ChallengeField,
    Option<F::ChallengeField>,
) {
    let timer = Timer::new("gkr_par_verifier_verify", true);
    let sp = VerifierScratchPad::<F>::new(circuit, proving_time_mpi_size);

    let mut challenge = ExpanderSingleVarChallenge::sample_from_transcript(
        transcript,
        circuit.layers.last().unwrap().output_var_num,
        proving_time_mpi_size,
    ).into();

    let transcript_state = transcript.hash_and_return_state();
    let verification_exec_units = parse_proof(circuit, &mut proof_reader, proving_time_mpi_size);

    let init_state = &verification_exec_units.first().unwrap().2;
    let mut verified = init_state.transcript_state == transcript_state
        && init_state.challenge == challenge
        && init_state.alpha.is_none()
        && init_state.claimed_v0 == *claimed_v
        && init_state.claimed_v1.is_none();

    let sumcheck_finished_states = verification_exec_units
        .par_iter()
        .enumerate()
        .map(|(i, (layer, proof_bytes, state))| {
            let mut sp = sp.clone();
            let mut local_transcript = T::new();
            let mut state = state.clone();
            local_transcript.set_state(&state.transcript_state);

            let mut cursor = Cursor::new(proof_bytes);
            let verified =
                sumcheck_verify_gkr_layer(
                    GKRScheme::GKRParVerifier,
                    proving_time_mpi_size,
                    layer,
                    public_input,
                    &mut state.challenge,
                    &mut state.claimed_v0,
                    &mut state.claimed_v1,
                    state.alpha,
                    &mut cursor,
                    &mut local_transcript,
                    &mut sp,
                    i == 0,
                );

            let alpha = if state.challenge.rz_1.is_some() {
                Some(local_transcript.generate_challenge_field_element())
            } else {
                None
            };

            (
                verified,
                SumcheckLayerState::<F> {
                    transcript_state: local_transcript.hash_and_return_state(),
                    challenge: state.challenge,
                    alpha,
                    claimed_v0: state.claimed_v0,
                    claimed_v1: state.claimed_v1,
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
        final_state.1.challenge.clone(),
        final_state.1.claimed_v0,
        final_state.1.claimed_v1,
    )
}
