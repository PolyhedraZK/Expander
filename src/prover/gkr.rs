//! This module implements the core GKR IOP.

use arith::{FiatShamirConfig, Field, FieldSerde, MultiLinearPoly};
use ark_std::{end_timer, start_timer};

use crate::{sumcheck_prove_gkr_layer, Circuit, Config, GkrScratchpad, Transcript};

// FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_prove<F>(
    circuit: &Circuit<F>,
    sp: &mut [GkrScratchpad<F>],
    transcript: &mut Transcript,
    config: &Config,
) -> (
    Vec<F>,
    Vec<Vec<F::ChallengeField>>,
    Vec<Vec<F::ChallengeField>>,
)
where
    F: Field + FieldSerde + FiatShamirConfig,
{
    let timer = start_timer!(|| "gkr prove");
    let layer_num = circuit.layers.len();

    let mut rz0 = vec![vec![]; config.get_num_repetitions()];
    let mut rz1 = vec![vec![]; config.get_num_repetitions()];
    log::trace!("repetition: {}", config.get_num_repetitions());
    for _i in 0..circuit.layers.last().unwrap().output_var_num {
        for j in 0..config.get_num_repetitions() {
            rz0[j].push(transcript.challenge_f::<F>());
            rz1[j].push(F::ChallengeField::zero());
        }
    }

    let mut alpha = F::ChallengeField::one();
    let mut beta = F::ChallengeField::zero();
    let mut claimed_v = vec![];

    for t in rz0.iter().take(config.get_num_repetitions()) {
        claimed_v.push(MultiLinearPoly::<F>::eval_multilinear(
            &circuit.layers.last().unwrap().output_vals.evals,
            t,
        ))
    }

    for i in (0..layer_num).rev() {
        (rz0, rz1) = sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            &alpha,
            &beta,
            transcript,
            sp,
            config,
        );
        alpha = transcript.challenge_f::<F>();
        beta = transcript.challenge_f::<F>();

        log::trace!("Layer {} proved with alpha={:?}, beta={:?}", i, alpha, beta);
        log::trace!("rz0.0: {:?}", rz0[0]);
        log::trace!("rz0.1: {:?}", rz0[1]);
        log::trace!("rz0.2: {:?}", rz0[2]);
        log::trace!("rz1.0: {:?}", rz1[0]);
        log::trace!("rz1.1: {:?}", rz1[1]);
        log::trace!("rz1.2: {:?}", rz1[2]);
    }

    end_timer!(timer);
    (claimed_v, rz0, rz1)
}
