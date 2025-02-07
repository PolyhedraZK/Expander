//! This module implements the core GKR IOP.

use arith::{Field, SimdField};
use circuit::Circuit;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultiLinearPoly;
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

    let claimed_v_simd = C::eval_circuit_vals_at_challenge(output_vals, &rz0, &mut sp.hg_evals);
    let claimed_v_local = MultiLinearPoly::<C::ChallengeField>::evaluate_with_buffer(
        &claimed_v_simd.unpack(),
        &r_simd,
        &mut sp.eq_evals_at_r_simd0,
    );

    let claimed_v = if mpi_config.is_root() {
        let mut claimed_v_gathering_buffer =
            vec![C::ChallengeField::zero(); mpi_config.world_size()];
        mpi_config.gather_vec(&vec![claimed_v_local], &mut claimed_v_gathering_buffer);
        MultiLinearPoly::evaluate_with_buffer(
            &claimed_v_gathering_buffer,
            &r_mpi,
            &mut sp.eq_evals_at_r_mpi0,
        )
    } else {
        mpi_config.gather_vec(&vec![claimed_v_local], &mut vec![]);
        C::ChallengeField::zero()
    };

    for i in (0..layer_num).rev() {
        let timer = Timer::new(
            &format!(
                "Sumcheck Layer {}, n_vars {}, one phase only? {}",
                i,
                &circuit.layers[i].input_var_num,
                &circuit.layers[i].structure_info.max_degree_one
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
