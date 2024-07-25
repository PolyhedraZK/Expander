// an implementation of the GKR^2 protocol
//! This module implements the core GKR^2 IOP.

use arith::MultiLinearPoly;
use ark_std::{end_timer, start_timer};

use crate::{sumcheck_prove_gkr_square_layer, Circuit, GKRConfig, GkrScratchpad, Transcript};

pub fn gkr_square_prove<C: GKRConfig>(
    circuit: &Circuit<C>,
    sp: &mut GkrScratchpad<C>,
    transcript: &mut Transcript,
) -> (C::Field, Vec<C::ChallengeField>) {
    let timer = start_timer!(|| "gkr^2 prove");
    let layer_num = circuit.layers.len();

    let mut rz0 = vec![];
    for _i in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.challenge_f::<C>());
    }

    let claimed_v = MultiLinearPoly::<C::Field>::eval_multilinear(
        &circuit.layers.last().unwrap().output_vals.evals,
        &rz0,
    );

    for i in (0..layer_num).rev() {
        rz0 = sumcheck_prove_gkr_square_layer(&circuit.layers[i], &rz0, transcript, sp);

        log::trace!("Layer {} proved", i);
        log::trace!("rz0.0: {:?}", rz0[0]);
        log::trace!("rz0.1: {:?}", rz0[1]);
        log::trace!("rz0.2: {:?}", rz0[2]);
    }

    end_timer!(timer);
    (claimed_v, rz0)
}
