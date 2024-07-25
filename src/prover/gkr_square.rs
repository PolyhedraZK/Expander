// an implementation of the GKR^2 protocol
//! This module implements the core GKR^2 IOP.

use arith::{Field, FieldSerde, MultiLinearPoly, SimdField};
use ark_std::{end_timer, start_timer};

use crate::{sumcheck_prove_gkr_square_layer, Circuit, Config, GkrScratchpad, Transcript};

// FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_square_prove<F>(
    circuit: &Circuit<F>,
    sp: &mut GkrScratchpad<F>,
    transcript: &mut Transcript,
    config: &Config,
) -> (F, Vec<F::Scalar>)
where
    F: Field + FieldSerde + SimdField,
{
    let timer = start_timer!(|| "gkr^2 prove");
    let layer_num = circuit.layers.len();

    let mut rz0 = vec![];
    for _i in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.challenge_f::<F>());
    }

    let claimed_v = MultiLinearPoly::<F>::eval_multilinear(
        &circuit.layers.last().unwrap().output_vals.evals,
        &rz0,
    );

    for i in (0..layer_num).rev() {
        rz0 = sumcheck_prove_gkr_square_layer(&circuit.layers[i], &rz0, transcript, sp, config);

        log::trace!("Layer {} proved", i);
        log::trace!("rz0.0: {:?}", rz0[0]);
        log::trace!("rz0.1: {:?}", rz0[1]);
        log::trace!("rz0.2: {:?}", rz0[2]);
    }

    end_timer!(timer);
    (claimed_v, rz0)
}
