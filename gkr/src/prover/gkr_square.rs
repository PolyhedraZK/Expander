// an implementation of the GKR^2 protocol
//! This module implements the core GKR^2 IOP.

use ark_std::{end_timer, start_timer};
use circuit::Circuit;
use gkr_field_config::{FieldType, GKRFieldConfig};
use mpi_config::MPIConfig;
use polynomials::MultiLinearPolyExpander;
use sumcheck::{sumcheck_prove_gkr_square_layer, ProverScratchPad};
use transcript::Transcript;

#[allow(clippy::type_complexity)]
pub fn gkr_square_prove<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    circuit: &Circuit<C>,
    sp: &mut ProverScratchPad<C>,
    transcript: &mut T,
    mpi_config: &MPIConfig,
) -> (
    C::ChallengeField,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
) {
    assert_ne!(
        C::FIELD_TYPE,
        FieldType::GF2,
        "GF2 is not supported in GKR^2"
    );
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

    let mut r_mpi = vec![];
    for _ in 0..mpi_config.world_size().trailing_zeros() {
        r_mpi.push(transcript.generate_challenge_field_element());
    }

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

    log::trace!("Claimed v: {:?}", claimed_v);

    for i in (0..layer_num).rev() {
        (rz0, r_simd, r_mpi) = sumcheck_prove_gkr_square_layer(
            &circuit.layers[i],
            &rz0,
            &r_simd,
            &r_mpi,
            transcript,
            sp,
            mpi_config,
        );

        log::trace!("Layer {} proved", i);
        log::trace!("rz0.0: {:?}", rz0[0]);
        log::trace!("rz0.1: {:?}", rz0[1]);
        log::trace!("rz0.2: {:?}", rz0[2]);
    }

    end_timer!(timer);
    (claimed_v, rz0, r_simd, r_mpi)
}
