// an implementation of the GKR^2 protocol
//! This module implements the core GKR^2 IOP.

use std::io::Read;

use ark_std::{end_timer, start_timer};
use circuit::Circuit;
use gkr_engine::{
    ExpanderSingleVarChallenge, FieldEngine, FieldType, MPIConfig, MPIEngine, Transcript,
};
use sumcheck::{
    ProverScratchPad, VerifierScratchPad, sumcheck_prove_gkr_square_layer,
    sumcheck_verify_gkr_square_layer,
};

#[allow(clippy::type_complexity)]
pub fn gkr_square_prove<F: FieldEngine>(
    circuit: &Circuit<F>,
    sp: &mut ProverScratchPad<F>,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mpi_config: &MPIConfig,
) -> (F::ChallengeField, ExpanderSingleVarChallenge<F>) {
    assert_ne!(
        F::FIELD_TYPE,
        FieldType::GF2,
        "GF2 is not supported in GKR^2"
    );
    let timer = start_timer!(|| "gkr^2 prove");
    let layer_num = circuit.layers.len();

    let mut challenge = ExpanderSingleVarChallenge::sample_from_transcript(
        transcript,
        circuit.layers.last().unwrap().output_var_num,
        mpi_config.world_size(),
    );

    let output_vals = &circuit.layers.last().unwrap().output_vals;
    let claimed_v = F::collectively_eval_circuit_vals_at_expander_challenge(
        output_vals,
        &challenge,
        &mut sp.hg_evals,
        &mut sp.eq_evals_first_half, // confusing name here..
        mpi_config,
    );

    log::trace!("Claimed v: {:?}", claimed_v);

    for i in (0..layer_num).rev() {
        sumcheck_prove_gkr_square_layer(
            &circuit.layers[i],
            &mut challenge,
            transcript,
            sp,
            mpi_config,
        );

        log::trace!("Layer {} proved", i);
        log::trace!("rz0.0: {:?}", challenge.rz[0]);
        log::trace!("rz0.1: {:?}", challenge.rz[1]);
        log::trace!("rz0.2: {:?}", challenge.rz[2]);
    }

    end_timer!(timer);
    (claimed_v, challenge)
}

#[allow(clippy::type_complexity)]
pub fn gkr_square_verify<C: FieldEngine>(
    proving_time_mpi_size: usize,
    circuit: &Circuit<C>,
    public_input: &[C::SimdCircuitField],
    claimed_v: &C::ChallengeField,
    transcript: &mut impl Transcript<C::ChallengeField>,
    mut proof_reader: impl Read,
) -> (bool, ExpanderSingleVarChallenge<C>, C::ChallengeField) {
    assert_ne!(
        C::FIELD_TYPE,
        FieldType::GF2,
        "GF2 is not supported in GKR^2"
    );

    let timer = start_timer!(|| "gkr verify");
    let mut sp = VerifierScratchPad::<C>::new(circuit, proving_time_mpi_size);

    let layer_num = circuit.layers.len();

    let mut challenge = ExpanderSingleVarChallenge::sample_from_transcript(
        transcript,
        circuit.layers.last().unwrap().output_var_num,
        proving_time_mpi_size,
    );

    log::trace!("Initial rz0: {:?}", challenge.rz);
    log::trace!("Initial r_simd: {:?}", challenge.r_simd);
    log::trace!("Initial r_mpi: {:?}", challenge.r_mpi);

    let mut verified = true;
    let mut current_claim = *claimed_v;
    log::trace!("Starting claim: {:?}", current_claim);
    for i in (0..layer_num).rev() {
        let cur_verified;
        (cur_verified, challenge, current_claim) = sumcheck_verify_gkr_square_layer(
            proving_time_mpi_size,
            &circuit.layers[i],
            public_input,
            &challenge,
            current_claim,
            &mut proof_reader,
            transcript,
            &mut sp,
            i == layer_num - 1,
        );
        log::trace!("Layer {} verified? {}", i, cur_verified);
        verified &= cur_verified;
    }
    end_timer!(timer);
    (verified, challenge, current_claim)
}
