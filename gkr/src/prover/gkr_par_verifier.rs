use crate::verifier::SumcheckLayerState;
use circuit::Circuit;
use gkr_engine::{
    ExpanderDualVarChallenge, ExpanderSingleVarChallenge, FieldEngine, MPIEngine, Transcript,
};
use serdes::ExpSerde;
use sumcheck::{sumcheck_prove_gkr_layer, ProverScratchPad};
use utils::timer::Timer;

#[allow(clippy::too_many_arguments)]
pub fn checkpoint_sumcheck_layer_state<F: FieldEngine>(
    challenge: &ExpanderDualVarChallenge<F>,
    alpha: &Option<F::ChallengeField>,
    claimed_v0: &F::ChallengeField,
    claimed_v1: &Option<F::ChallengeField>,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mpi_config: &impl MPIEngine,
) {
    let transcript_state = transcript.hash_and_return_state();
    if mpi_config.is_root() {
        let sumcheck_state = SumcheckLayerState::<F> {
            transcript_state: transcript_state.clone(),
            challenge: challenge.clone(),
            alpha: *alpha,
            claimed_v0: *claimed_v0,
            claimed_v1: *claimed_v1,
        };
        let mut state_bytes = vec![];
        sumcheck_state.serialize_into(&mut state_bytes).unwrap();
        transcript.append_u8_slice(&state_bytes);
        transcript.set_state(&transcript_state);
    }
}

#[allow(clippy::type_complexity)]
pub fn gkr_par_verifier_prove<F: FieldEngine>(
    circuit: &Circuit<F>,
    sp: &mut ProverScratchPad<F>,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mpi_config: &impl MPIEngine,
) -> (F::ChallengeField, ExpanderDualVarChallenge<F>) {
    let layer_num = circuit.layers.len();

    let mut challenge: ExpanderDualVarChallenge<F> =
        ExpanderSingleVarChallenge::sample_from_transcript(
            transcript,
            circuit.layers.last().unwrap().output_var_num,
            mpi_config.world_size(),
        )
        .into();
    let mut alpha = None;

    let output_vals = &circuit.layers.last().unwrap().output_vals;
    let claimed_v = F::collectively_eval_circuit_vals_at_expander_challenge(
        output_vals,
        &challenge.challenge_x(),
        &mut sp.hg_evals,
        &mut sp.eq_evals_first_half, // confusing name here..
        mpi_config,
    );

    let mut claimed_v0 = claimed_v;
    let mut claimed_v1 = None;
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

        checkpoint_sumcheck_layer_state::<F>(
            &challenge,
            &alpha,
            &claimed_v0,
            &claimed_v1,
            transcript,
            mpi_config,
        );

        (claimed_v0, claimed_v1) = sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &mut challenge,
            alpha,
            transcript,
            sp,
            mpi_config,
            i == layer_num - 1,
        );

        if challenge.rz_1.is_some() {
            let mut tmp = transcript.generate_challenge_field_element();
            mpi_config.root_broadcast_f(&mut tmp);
            alpha = Some(tmp)
        } else {
            alpha = None;
        }
        timer.stop();
    }

    transcript.hash_and_return_state(); // trigger an additional hash to compress all the unhashed data, for ease of verification
    (claimed_v, challenge)
}
