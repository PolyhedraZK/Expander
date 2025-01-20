use arith::ExtensionField;
use gkr_field_config::GKRFieldConfig;
use transcript::Transcript;

use crate::{
    CrossLayerCircuitEvals, CrossLayerConnections, CrossLayerGatherHelper,
    CrossLayerProverScratchPad, CrossLayerScatterHelper, GenericLayer,
};

#[inline]
pub fn transcript_io<F: ExtensionField, T: Transcript<F>>(ps: &[F], transcript: &mut T) -> F {
    assert!(ps.len() == 3 || ps.len() == 4);
    for p in ps {
        transcript.append_field_element(p);
    }
    transcript.generate_challenge_field_element()
}

// FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn sumcheck_prove_scatter_layer<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    layer: &GenericLayer<C>,
    rz0: &[C::ChallengeField],
    r_simd: &[C::ChallengeField],
    connections: &CrossLayerConnections,
    circuit_vals: &CrossLayerCircuitEvals<C>,
    transcript: &mut T,
    sp: &mut CrossLayerProverScratchPad<C>,
) -> (
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<(usize, Vec<C::ChallengeField>)>,
) {
    let mut helper =
        CrossLayerScatterHelper::new(layer, rz0, r_simd, connections, circuit_vals, sp);

    helper.prepare_simd();

    // gkr phase 1 over variable x
    helper.prepare_x_vals();
    for i_var in 0..helper.input_layer_var_num {
        let evals = helper.poly_evals_at_rx(i_var, 2);
        let r = transcript_io::<C::ChallengeField, T>(&evals, transcript);
        helper.receive_rx(i_var, r);
    }

    helper.prepare_simd_var_vals();
    for i_var in 0..C::get_field_pack_size().trailing_zeros() as usize {
        let evals = helper.poly_evals_at_r_simd_var(i_var, 3);
        let r = transcript_io::<C::ChallengeField, T>(&evals, transcript);
        helper.receive_r_simd_var(i_var, r);
    }

    let vx_claims: Vec<(usize, C::ChallengeField)> = helper.vx_claims();
    for (_i_layer, claim) in vx_claims {
        transcript.append_field_element(&claim);
    }

    // gkr phase 2 over variable y
    helper.prepare_y_vals();
    for i_var in 0..helper.input_layer_var_num {
        let evals = helper.poly_evals_at_ry(i_var, 2);
        let r = transcript_io::<C::ChallengeField, T>(&evals, transcript);
        helper.receive_ry(i_var, r);
    }
    let vy_claim = helper.vy_claim();
    transcript.append_field_element(&vy_claim);

    (helper.rx, helper.ry, helper.r_relays_next)
}

#[allow(clippy::too_many_arguments)]
pub fn sumcheck_prove_gather_layer<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    layer: &GenericLayer<C>,
    rz0: &[C::ChallengeField],
    rz1: &[C::ChallengeField],
    r_relays: &[(usize, Vec<C::ChallengeField>)],
    connections: &CrossLayerConnections,
    circuit_vals: &CrossLayerCircuitEvals<C>,
    transcript: &mut T,
    sp: &mut CrossLayerProverScratchPad<C>,
) -> (Vec<C::ChallengeField>, C::ChallengeField) {
    let alpha = transcript.generate_challenge_field_element();
    let betas = transcript.generate_challenge_field_elements(r_relays.len());

    let mut helper = CrossLayerGatherHelper::new(
        layer,
        rz0,
        rz1,
        r_relays,
        &alpha,
        &betas,
        connections,
        circuit_vals,
        sp,
    );

    helper.prepare_x_vals();
    for i_var in 0..helper.cur_layer_var_num {
        let evals = helper.poly_evals_at_rx(i_var, 2);
        let r = transcript_io::<C::ChallengeField, T>(&evals, transcript);
        helper.receive_rx(i_var, r);
    }

    let vx_claim = helper.vx_claim();
    transcript.append_field_element(&vx_claim);

    // No variable Y in gathering layer

    (helper.rx, vx_claim)
}
