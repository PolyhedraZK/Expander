//! This module implements the core GKR IOP.

use arith::{Field, SimdField};
use ark_std::{end_timer, start_timer};

use crate::{
    sumcheck_prove_gkr_layer, Circuit, GKRConfig, GkrScratchpad, MultiLinearPoly, Transcript,
};

// FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_prove<C: GKRConfig>(
    circuit: &Circuit<C>,
    sp: &mut GkrScratchpad<C>,
    transcript: &mut Transcript<C::FiatShamirHashType>,
) -> (
    C::ChallengeField,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
) {
    let timer = start_timer!(|| "gkr prove");
    let layer_num = circuit.layers.len();

    let mut rz0 = vec![];
    let mut rz1 = vec![];
    let mut r_simd = vec![];
    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.challenge_f::<C>());
        rz1.push(C::ChallengeField::zero());
    }

    for _ in 0..C::get_field_pack_size().trailing_zeros() {
        r_simd.push(transcript.challenge_f::<C>());
    }

    let mut alpha = C::ChallengeField::one();
    let mut beta = C::ChallengeField::zero();

    let output_vals = &circuit.layers.last().unwrap().output_vals;
    let claimed_v_simd =
        MultiLinearPoly::eval_circuit_vals_at_challenge::<C>(output_vals, &rz0, &mut sp.hg_evals);
    let claimed_v = MultiLinearPoly::eval_generic::<C::ChallengeField>(
        &claimed_v_simd.unpack(),
        &r_simd,
        &mut sp.eq_evals_at_r_simd0,
    );

    for i in (0..layer_num).rev() {
        (rz0, rz1, r_simd) = sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            &r_simd,
            &alpha,
            &beta,
            transcript,
            sp,
        );
        alpha = transcript.challenge_f::<C>();
        beta = transcript.challenge_f::<C>();

        log::trace!("Layer {} proved with alpha={:?}, beta={:?}", i, alpha, beta);
        log::trace!("rz0.0: {:?}", rz0[0]);
        log::trace!("rz0.1: {:?}", rz0[1]);
        log::trace!("rz0.2: {:?}", rz0[2]);
        log::trace!("rz1.0: {:?}", rz1[0]);
        log::trace!("rz1.1: {:?}", rz1[1]);
        log::trace!("rz1.2: {:?}", rz1[2]);
    }

    end_timer!(timer);
    (claimed_v, rz0, rz1, r_simd)
}
