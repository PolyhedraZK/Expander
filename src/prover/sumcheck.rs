use arith::{Field, FieldSerde, VectorizedField};

use crate::{CircuitLayer, Config, GkrScratchpad, SumcheckGkrHelper, Transcript};

// FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn sumcheck_prove_gkr_layer<F>(
    layer: &CircuitLayer<F>,
    rz0: &[Vec<F::BaseField>],
    rz1: &[Vec<F::BaseField>],
    alpha: &F::BaseField,
    beta: &F::BaseField,
    transcript: &mut Transcript,
    sp: &mut [GkrScratchpad<F>],
    config: &Config,
) -> (Vec<Vec<F::BaseField>>, Vec<Vec<F::BaseField>>)
where
    F: VectorizedField + FieldSerde,
    F::PackedBaseField: Field<BaseField = F::BaseField>,
{
    println!("sumcheck 1");
    let mut helpers = vec![];
    assert_eq!(config.get_num_repetitions(), sp.len());
    for (j, sp_) in sp.iter_mut().enumerate() {
        helpers.push(SumcheckGkrHelper::new(
            layer, &rz0[j], &rz1[j], alpha, beta, sp_,
        ));
    }

    println!("sumcheck 2");
    for i_var in 0..layer.input_var_num * 2 {
        for (j, helper) in helpers
            .iter_mut()
            .enumerate()
            .take(config.get_num_repetitions())
        {
            if i_var == 0 {
                helper.prepare_g_x_vals()
            }
            if i_var == layer.input_var_num {
                let vx_claim = helper.vx_claim();
                helper.prepare_h_y_vals(vx_claim)
            }

            println!("sumcheck 2.1");
            let evals = helper.poly_evals_at(i_var, 2);

            println!("sumcheck 2.2");
            transcript.append_f(evals[0]);
            transcript.append_f(evals[1]);
            transcript.append_f(evals[2]);
            println!("sumcheck 2.3");
            let r = transcript.challenge_f::<F>();
            println!("sumcheck 2.4");
            println!("i_var={} j={} evals: {:?} r: {:?}", i_var, j, evals, r);
            if j == 0 {
                log::trace!("i_var={} j={} evals: {:?} r: {:?}", i_var, j, evals, r);
            }
            helper.receive_challenge(i_var, r);
            if i_var == layer.input_var_num - 1 {
                log::trace!("vx claim: {:?}", helper.vx_claim());
                transcript.append_f(helper.vx_claim());
            }
        }
    }

    println!("sumcheck 3");
    for (j, helper) in helpers
        .iter()
        .enumerate()
        .take(config.get_num_repetitions())
    {
        log::trace!("claimed vy[{}] = {:?}", j, helper.vy_claim());
        transcript.append_f(helper.vy_claim());
    }

    println!("sumcheck 4");
    let rz0s = (0..config.get_num_repetitions())
        .map(|j| helpers[j].rx.clone()) // FIXME: clone might be avoided
        .collect();
    let rz1s = (0..config.get_num_repetitions())
        .map(|j| helpers[j].ry.clone()) // FIXME: clone might be avoided
        .collect();
    (rz0s, rz1s)
}
