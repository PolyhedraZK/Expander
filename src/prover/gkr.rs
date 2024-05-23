use arith::{Field, MultiLinearPoly, VectorizedM31, M31};

use crate::{sumcheck_prove_gkr_layer, Circuit, Config, GkrScratchpad, Transcript};

type FPrimitive = M31;
type F = VectorizedM31;

pub fn gkr_prove(
    circuit: &Circuit,
    sp: &mut [GkrScratchpad],
    transcript: &mut Transcript,
    config: &Config,
) -> (Vec<F>, Vec<Vec<FPrimitive>>, Vec<Vec<FPrimitive>>) {
    let layer_num = circuit.layers.len();

    let mut rz0 = vec![vec![]; config.get_num_repetitions()];
    let mut rz1 = vec![vec![]; config.get_num_repetitions()];

    for i in 0..circuit.layers.last().unwrap().output_var_num {
        for j in 0..config.get_num_repetitions() {
            rz0[j].push(transcript.challenge_f());
            rz1[j].push(FPrimitive::zero());
        }
    }
    let mut alpha = FPrimitive::one();
    let mut beta = FPrimitive::zero();
    let mut claimed_v = vec![];

    for j in 0..config.get_num_repetitions() {
        claimed_v.push(MultiLinearPoly::<F>::eval_multilinear(
            &circuit.layers.last().unwrap().output_vals.evals,
            &rz0[j],
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
        alpha = transcript.challenge_f();
        beta = transcript.challenge_f();
        // println!("Layer {} proved with alpha={:?}, beta={:?}", i, alpha, beta);
        // println!("rz0.0: {:?}", rz0[0]);
        // println!("rz0.1: {:?}", rz0[1]);
        // println!("rz0.2: {:?}", rz0[2]);
        // println!("rz1.0: {:?}", rz1[0]);
        // println!("rz1.1: {:?}", rz1[1]);
        // println!("rz1.2: {:?}", rz1[2]);
    }

    (claimed_v, rz0, rz1)
}
