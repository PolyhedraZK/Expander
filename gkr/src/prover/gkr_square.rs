// an implementation of the GKR^2 protocol
//! This module implements the core GKR^2 IOP.

use arith::{Field, SimdField};
use ark_std::{end_timer, start_timer};
use circuit::Circuit;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultiLinearPoly;
use sumcheck::{sumcheck_prove_gkr_square_layer, ProverScratchPad};
use transcript::Transcript;

pub fn gkr_square_prove<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    circuit: &Circuit<C>,
    sp: &mut ProverScratchPad<C>,
    transcript: &mut T,
    mpi_config: &MPIConfig,
) -> (
    C::ChallengeField,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
) {
    let timer = start_timer!(|| "gkr^2 prove");
    let layer_num = circuit.layers.len();

    let mut rz0 = vec![];
    for _i in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.generate_challenge_field_element());
    }

    let mut r_simd = vec![];
    for _i in 0..C::get_field_pack_size().trailing_zeros() {
        r_simd.push(transcript.generate_challenge_field_element());
    }
    log::trace!("Initial r_simd: {:?}", r_simd);

    // TODO: MPI support
    assert_eq!(
        mpi_config.world_size().trailing_zeros(),
        0,
        "MPI not supported yet"
    );
    let mut r_mpi = vec![];
    for _ in 0..mpi_config.world_size().trailing_zeros() {
        r_mpi.push(transcript.generate_challenge_field_element());
    }

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
    log::trace!("Claimed v: {:?}", claimed_v);

    for i in (0..layer_num).rev() {
        (rz0, r_simd) =
            sumcheck_prove_gkr_square_layer(&circuit.layers[i], &rz0, &r_simd, transcript, sp);

        log::trace!("Layer {} proved", i);
        log::trace!("rz0.0: {:?}", rz0[0]);
        log::trace!("rz0.1: {:?}", rz0[1]);
        log::trace!("rz0.2: {:?}", rz0[2]);
    }

    end_timer!(timer);
    (claimed_v, rz0, r_simd)
}
