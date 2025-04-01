use std::{io::Read, vec};

use arith::Field;
use circuit::Circuit;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use serdes::ExpSerde;
use sumcheck::{VerifierScratchPad, SUMCHECK_GKR_DEGREE, SUMCHECK_GKR_SIMD_MPI_DEGREE};
use transcript::Transcript;
use utils::timer::Timer;

use crate::prover::gkr_par_verifier::SumcheckLayerState;
use super::common::sumcheck_verify_gkr_layer;

pub fn parse_proof<C: GKRFieldConfig>(
    circuit: &Circuit<C>,
    mut proof_reader: impl Read,
    mpi_config: &MPIConfig,
) -> Vec<(Vec<u8>, SumcheckLayerState<C>)> {
    circuit.layers
        .iter()
        .rev()
        .map(| layer| {
            let sumcheck_layer_state = SumcheckLayerState::<C>::deserialize_from(&mut proof_reader).unwrap();
            let proof_size_n_challenge_fields = 
                layer.input_var_num * (!layer.structure_info.skip_sumcheck_phase_two as usize + 1) * (SUMCHECK_GKR_DEGREE + 1) // variable x, y
                + (C::get_field_pack_size().ilog2() as usize + mpi_config.world_size().ilog2() as usize) * (SUMCHECK_GKR_SIMD_MPI_DEGREE + 1); // variable simd, mpi
            let proof_size_n_bytes = proof_size_n_challenge_fields * C::ChallengeField::SIZE;
            let mut proof_bytes = vec![0u8; proof_size_n_bytes];
            proof_reader.read_exact(&mut proof_bytes).unwrap();

            (
                proof_bytes,
                sumcheck_layer_state,
            )
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
    let mut sp = VerifierScratchPad::<C>::new(circuit, mpi_config.world_size());

    let layer_num = circuit.layers.len();
    let mut rz0 = vec![];
    let mut rz1 = None;
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

    let mut alpha = None;
    let mut claimed_v0 = *claimed_v;
    let mut claimed_v1 = None;

    let mut verified = true;
    let verification_units: Vec<(Vec<u8>, SumcheckLayerState<C>)> = parse_proof(
        circuit,
        &mut proof_reader,
        mpi_config,
    );

    for i in (0..layer_num).rev() {
        let cur_verified;
        (
            cur_verified,
            rz0,
            rz1,
            r_simd,
            r_mpi,
            claimed_v0,
            claimed_v1,
        ) = sumcheck_verify_gkr_layer(
            mpi_config,
            &circuit.layers[i],
            public_input,
            &rz0,
            &rz1,
            &r_simd,
            &r_mpi,
            claimed_v0,
            claimed_v1,
            alpha,
            &mut proof_reader,
            transcript,
            &mut sp,
            i == layer_num - 1,
        );

        verified &= cur_verified;
        alpha = if rz1.is_some() {
            Some(transcript.generate_challenge_field_element())
        } else {
            None
        };
    }
    timer.stop();
    (verified, rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1)
}
