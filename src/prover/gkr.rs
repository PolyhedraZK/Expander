use crate::{
    eval_multilinear, sumcheck_prove_gkr_layer, Circuit, Config, Field, GkrScratchpad, Transcript,
    VectorizedM31, M31,
};

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
        claimed_v.push(eval_multilinear(
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
            &config,
        );
        alpha = transcript.challenge_f();
        beta = transcript.challenge_f();
        // println!("Layer {} proved with alpha={:?}, beta={:?}", i, alpha, beta);
    }

    (claimed_v, rz0, rz1)
}
