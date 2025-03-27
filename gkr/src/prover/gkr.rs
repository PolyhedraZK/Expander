//! This module implements the core GKR IOP.

use circuit::Circuit;
use gkr_engine::{ExpanderDualVarChallenge, FieldEngine, MPIConfig, MPIEngine, Transcript};
use sumcheck::{sumcheck_prove_gkr_layer, ProverScratchPad};
use utils::timer::Timer;

// FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_prove<F: FieldEngine>(
    circuit: &Circuit<F>,
    sp: &mut ProverScratchPad<F>,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mpi_config: &MPIConfig,
) -> (F::ChallengeField, ExpanderDualVarChallenge<F>) {
    let layer_num = circuit.layers.len();

    let mut challenge = ExpanderDualVarChallenge::empty();

    challenge.rz_0 =
        transcript.generate_challenge_field_elements(circuit.layers.last().unwrap().output_var_num);

    challenge.r_simd = transcript
        .generate_challenge_field_elements(F::get_field_pack_size().trailing_zeros() as usize);

    challenge.r_mpi = transcript
        .generate_challenge_field_elements(mpi_config.world_size().trailing_zeros() as usize);

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
