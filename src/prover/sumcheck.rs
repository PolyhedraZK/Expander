use crate::{
    CircuitLayer, GKRConfig, GkrScratchpad, SumcheckGkrHelper, SumcheckGkrSquareHelper, Transcript,
};

// FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn sumcheck_prove_gkr_layer<C: GKRConfig>(
    layer: &CircuitLayer<C>,
    rz0: &[C::ChallengeField],
    rz1: &[C::ChallengeField],
    alpha: &C::ChallengeField,
    beta: &C::ChallengeField,
    transcript: &mut Transcript,
    sp: &mut GkrScratchpad<C>,
) -> (Vec<C::ChallengeField>, Vec<C::ChallengeField>) {
    let mut helper = SumcheckGkrHelper::new(layer, rz0, rz1, alpha, beta, sp);

    for i_var in 0..layer.input_var_num * 2 {
        if i_var == 0 {
            helper.prepare_g_x_vals()
        }

        if i_var == layer.input_var_num {
            let vx_claim = helper.vx_claim();
            helper.prepare_h_y_vals(vx_claim)
        }
        let evals = helper.poly_evals_at(i_var, 2);

        transcript.append_f::<C>(evals[0]);
        transcript.append_f::<C>(evals[1]);
        transcript.append_f::<C>(evals[2]);

        let r = transcript.challenge_f::<C>();
        println!("Prover var {} challenge r {:?}", i_var, r);

        log::trace!("i_var={} evals: {:?} r: {:?}", i_var, evals, r);

        helper.receive_challenge(i_var, r);
        if i_var == layer.input_var_num - 1 {
            log::trace!("vx claim: {:?}", helper.vx_claim());
            println!("alpha {:?}, beta {:?}, vx_claim {:?}", alpha, beta, helper.vx_claim());
            transcript.append_f::<C>(helper.vx_claim());
        }
    }

    log::trace!("claimed vy = {:?}", helper.vy_claim());
    transcript.append_f::<C>(helper.vy_claim());

    let rz0 = helper.rx.clone();
    let rz1 = helper.ry.clone();
    (rz0, rz1)
}

// FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
#[allow(clippy::needless_range_loop)] // todo: remove
pub fn sumcheck_prove_gkr_square_layer<C: GKRConfig>(
    layer: &CircuitLayer<C>,
    rz0: &[C::ChallengeField],
    transcript: &mut Transcript,
    sp: &mut GkrScratchpad<C>,
) -> Vec<C::ChallengeField> {
    const D: usize = 7;
    let mut helper = SumcheckGkrSquareHelper::new(layer, rz0, sp);

    for i_var in 0..layer.input_var_num {
        if i_var == 0 {
            helper.prepare_g_x_vals();
        }
        let evals: [C::Field; D] = helper.poly_evals_at(i_var);

        for deg in 0..D {
            transcript.append_f::<C>(evals[deg]);
        }

        let r = transcript.challenge_f::<C>();

        log::trace!("i_var={} evals: {:?} r: {:?}", i_var, evals, r);

        helper.receive_challenge(i_var, r);
        if i_var == layer.input_var_num - 1 {
            log::trace!("vx claim: {:?}", helper.vx_claim());
            transcript.append_f::<C>(helper.vx_claim());
        }
    }

    log::trace!("claimed vx = {:?}", helper.vx_claim());
    transcript.append_f::<C>(helper.vx_claim());

    helper.rx
}
