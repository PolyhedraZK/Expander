//! This module implements the core GKR IOP.

use std::io::Read;

use circuit::Circuit;
use gkr_engine::{ExpanderDualVarChallenge, FieldEngine, MPIConfig, MPIEngine, Transcript};
use utils::timer::Timer;

use crate::{
    sumcheck_prove_gkr_layer, sumcheck_verify_gkr_layer, ProverScratchPad, VerifierScratchPad,
};

// FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_prove<F: FieldEngine>(
    circuit: &Circuit<F>,
    sp: &mut ProverScratchPad<F>,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mpi_config: &MPIConfig,
) -> (F::ChallengeField, ExpanderDualVarChallenge<F>) {
    let layer_num = circuit.layers.len();

    let mut challenge = ExpanderDualVarChallenge::sample_from_transcript(
        transcript,
        circuit.layers.last().unwrap().output_var_num,
        mpi_config.world_size(),
    );

    let mut alpha = None;

    let output_vals = &circuit.layers.last().unwrap().output_vals;
    let claimed_v = F::collectively_eval_circuit_vals_at_expander_challenge(
        output_vals,
        &challenge.challenge_x(),
        &mut sp.hg_evals,
        &mut sp.eq_evals_first_half, // confusing name here..
        mpi_config,
    );

    for i in (0..layer_num).rev() {
        let timer = Timer::new(
            &format!(
                "Sumcheck Layer {}, n_vars {}, one phase only? {}",
                i,
                &circuit.layers[i].input_var_num,
                &circuit.layers[i].structure_info.skip_sumcheck_phase_two,
            ),
            mpi_config.is_root(),
        );

        sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &mut challenge,
            alpha,
            transcript,
            sp,
            mpi_config,
            i == layer_num - 1,
        );

        if challenge.rz_1.is_some() {
            // TODO: try broadcast beta.unwrap directly
            let mut tmp = transcript.generate_challenge_field_element();
            mpi_config.root_broadcast_f(&mut tmp);
            alpha = Some(tmp)
        } else {
            alpha = None;
        }
        timer.stop();
    }

    (claimed_v, challenge)
}

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_verify<F: FieldEngine>(
    proving_time_mpi_size: usize,
    circuit: &Circuit<F>,
    public_input: &[F::SimdCircuitField],
    claimed_v: &F::ChallengeField,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mut proof_reader: impl Read,
) -> (
    bool,
    ExpanderDualVarChallenge<F>,
    F::ChallengeField,
    Option<F::ChallengeField>,
) {
    let timer = Timer::new("gkr_verify", true);
    let mut sp = VerifierScratchPad::<F>::new(circuit, proving_time_mpi_size);

    let layer_num = circuit.layers.len();

    let mut challenge = ExpanderDualVarChallenge::sample_from_transcript(
        transcript,
        circuit.layers.last().unwrap().output_var_num,
        proving_time_mpi_size,
    );

    let mut alpha = None;
    let mut claimed_v0 = *claimed_v;
    let mut claimed_v1 = None;

    let mut verified = true;
    for i in (0..layer_num).rev() {
        let cur_verified = sumcheck_verify_gkr_layer(
            proving_time_mpi_size,
            &circuit.layers[i],
            public_input,
            &mut challenge,
            &mut claimed_v0,
            &mut claimed_v1,
            alpha,
            &mut proof_reader,
            transcript,
            &mut sp,
            i == layer_num - 1,
        );

        verified &= cur_verified;
        alpha = if challenge.rz_1.is_some() {
            Some(transcript.generate_challenge_field_element())
        } else {
            None
        };
    }
    timer.stop();
    let challenge = ExpanderDualVarChallenge::new(
        challenge.rz_0,
        challenge.rz_1,
        challenge.r_simd,
        challenge.r_mpi,
    );

    (verified, challenge, claimed_v0, claimed_v1)
}
