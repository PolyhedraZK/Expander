use arith::{FiatShamirConfig, Field, FieldSerde};

use crate::{CircuitLayer, Config, GkrScratchpad, SumcheckGkrHelper, Transcript};

// FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn sumcheck_prove_gkr_layer<F>(
    layer: &CircuitLayer<F>,
    rz0: &[F::ChallengeField],
    rz1: &[F::ChallengeField],
    alpha: &F::ChallengeField,
    beta: &F::ChallengeField,
    transcript: &mut Transcript,
    sp: &mut GkrScratchpad<F>,
    _config: &Config,
) -> (Vec<F::ChallengeField>, Vec<F::ChallengeField>)
where
    F: Field + FieldSerde + FiatShamirConfig,
{

    let mut helper = SumcheckGkrHelper::new(
        layer, &rz0, &rz1, alpha, beta, sp,
    );

    for i_var in 0..layer.input_var_num * 2 {
        if i_var == 0 {
            helper.prepare_g_x_vals()
        }

        if i_var == layer.input_var_num {
            let vx_claim = helper.vx_claim();
            helper.prepare_h_y_vals(vx_claim)
        }

        let evals = helper.poly_evals_at(i_var, 2);

        transcript.append_f(evals[0]);
        transcript.append_f(evals[1]);
        transcript.append_f(evals[2]);

        let r = transcript.challenge_f::<F>();
        
        log::trace!("i_var={} evals: {:?} r: {:?}", i_var, evals, r);
        
        helper.receive_challenge(i_var, r);
        if i_var == layer.input_var_num - 1 {
            log::trace!("vx claim: {:?}", helper.vx_claim());
            transcript.append_f(helper.vx_claim());
        }
    }

    log::trace!("claimed vy = {:?}", helper.vy_claim());
    transcript.append_f(helper.vy_claim());

    let rz0 = helper.rx.clone();
    let rz1 = helper.ry.clone();
    (rz0, rz1)
}
