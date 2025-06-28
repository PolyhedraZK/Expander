//! This module implements the core GKR IOP.

use circuit::Circuit;
use gkr_engine::{
    ExpanderDualVarChallenge, ExpanderSingleVarChallenge, FieldEngine, MPIConfig, MPIEngine,
    Transcript,
};
use sumcheck::{sumcheck_prove_gkr_layer, ProverScratchPad};
use utils::timer::Timer;

#[allow(clippy::type_complexity)]
pub fn gkr_prove<F: FieldEngine>(
    circuit: &Circuit<F>,
    sp: &mut ProverScratchPad<F>,
    transcript: &mut impl Transcript,
    mpi_config: &MPIConfig,
) -> (F::ChallengeField, ExpanderDualVarChallenge<F>)
where
    F::CircuitField: std::fmt::Debug,
    F::SimdCircuitField: std::fmt::Debug,
{
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

    // Serialize circuit to file if EXPANDER_GPU environment variable is set to 1
    if std::env::var("EXPANDER_GPU").is_ok_and(|v| v == "1") {
        if let Err(e) =
            gpu::serdes::serial_circuit_witness_as_plaintext(circuit, transcript, &challenge)
        {
            println!("Failed to serialize circuit: {e}");
        }
    }

    let mut final_vx_claim = None;
    let mut final_vy_claim = None;

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

        let (vx_claim, vy_claim) = sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &mut challenge,
            alpha,
            transcript,
            sp,
            mpi_config,
            i == layer_num - 1,
        );

        // Store the final layer claims for later use
        if i == 0 {
            final_vx_claim = Some(vx_claim);
            final_vy_claim = vy_claim;
        }

        if challenge.rz_1.is_some() {
            // TODO: try broadcast beta.unwrap directly
            let mut tmp = transcript.generate_field_element::<F::ChallengeField>();
            mpi_config.root_broadcast_f(&mut tmp);
            alpha = Some(tmp)
        } else {
            alpha = None;
        }
        timer.stop();
    }

    // Print final claims if EXPANDER_GPU environment variable is set to 1
    if std::env::var("EXPANDER_GPU").is_ok_and(|v| v == "1") {
        if let Some(vx) = final_vx_claim {
            gpu::serdes::print_final_claims::<F>(&vx, &final_vy_claim);
            println!("GKR final proof claims as shown above.");
            std::process::exit(0);
        }
    }

    (claimed_v, challenge)
}
