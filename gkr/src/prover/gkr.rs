//! This module implements the core GKR IOP.

use circuit::Circuit;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultiLinearPolyExpander;
use sumcheck::{sumcheck_prove_gkr_layer, ProverScratchPad};
use transcript::Transcript;
use utils::timer::Timer;

// FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_prove<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    circuit: &Circuit<C>,
    sp: &mut ProverScratchPad<C>,
    transcript: &mut T,
    mpi_config: &MPIConfig,
) -> (
    C::ChallengeField,
    Vec<C::ChallengeField>,
    Option<Vec<C::ChallengeField>>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
) {
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

    let output_vals = &circuit.layers.last().unwrap().output_vals;
    let claimed_v =
        MultiLinearPolyExpander::<C>::collectively_eval_circuit_vals_at_expander_challenge(
            output_vals,
            &rz0,
            &r_simd,
            &r_mpi,
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
        (rz0, rz1, r_simd, r_mpi) = sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            &r_simd,
            &r_mpi,
            alpha,
            transcript,
            sp,
            mpi_config,
            i == layer_num - 1,
        );

        if rz1.is_some() {
            // TODO: try broadcast beta.unwrap directly
            let mut tmp = transcript.generate_challenge_field_element();
            mpi_config.root_broadcast_f(&mut tmp);
            alpha = Some(tmp)
        } else {
            alpha = None;
        }
        timer.stop();
    }

    (claimed_v, rz0, rz1, r_simd, r_mpi)
}
