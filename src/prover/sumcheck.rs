use arith::{VectorizedM31, M31};

use crate::{CircuitLayer, Config, GkrScratchpad, SumcheckGkrHelper, Transcript};

type FPrimitive = M31;
type F = VectorizedM31;

pub fn sumcheck_prove_gkr_layer(
    layer: &CircuitLayer,
    rz0: &[Vec<FPrimitive>],
    rz1: &[Vec<FPrimitive>],
    alpha: &FPrimitive,
    beta: &FPrimitive,
    transcript: &mut Transcript,
    sp: &mut [GkrScratchpad],
    config: &Config,
) -> (Vec<Vec<FPrimitive>>, Vec<Vec<FPrimitive>>) {
    let mut helpers = vec![];
    assert_eq!(config.get_num_repetitions(), sp.len());
    for (j, sp_) in sp.iter_mut().enumerate() {
        helpers.push(SumcheckGkrHelper::new(
            layer, &rz0[j], &rz1[j], alpha, beta, sp_,
        ));
    }
    for i_var in 0..layer.input_var_num * 2 {
        for j in 0..config.get_num_repetitions() {
            if i_var == 0 {
                helpers[j].prepare_g_x_vals()
            }
            if i_var == layer.input_var_num {
                let vx_claim = helpers[j].vx_claim();
                helpers[j].prepare_h_y_vals(vx_claim)
            }
            let evals = helpers[j].poly_evals_at(i_var, 2);
            transcript.append_f(evals[0]);
            transcript.append_f(evals[1]);
            transcript.append_f(evals[2]);
            let r = transcript.challenge_f();

            // if j == 0 {
            //     println!("i_var={} j={} evals: {:?} r: {:?}", i_var, j, evals, r);
            // }
            helpers[j].receive_challenge(i_var, r);
            if i_var == layer.input_var_num - 1 {
                // println!("vx claim: {:?}", helpers[j].vx_claim());
                transcript.append_f(helpers[j].vx_claim());
            }
        }
    }
    for j in 0..config.get_num_repetitions() {
        // println!("claimed vy[{}] = {:?}", j, helpers[j].vy_claim());
        transcript.append_f(helpers[j].vy_claim());
    }

    let rz0s = (0..config.get_num_repetitions())
        .map(|j| helpers[j].rx.clone()) // FIXME: clone might be avoided
        .collect();
    let rz1s = (0..config.get_num_repetitions())
        .map(|j| helpers[j].ry.clone()) // FIXME: clone might be avoided
        .collect();
    (rz0s, rz1s)
}
