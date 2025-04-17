use arith::{Field, SimdField};
use gkr_engine::{FieldEngine, Transcript};
use polynomials::MultiLinearPoly;

use crate::sumcheck::{sumcheck_prove_gather_layer, sumcheck_prove_scatter_layer};
use crate::{circuit, scratchpad, CrossLayerCircuit};

pub fn prove_gkr<C: FieldEngine, T: Transcript>(
    circuit: &CrossLayerCircuit<C>,
    circuit_vals: &circuit::CrossLayerCircuitEvals<C>,
    connections: &circuit::CrossLayerConnections,
    transcript: &mut T,
    sp: &mut scratchpad::CrossLayerProverScratchPad<C>,
) -> (C::ChallengeField, Vec<C::ChallengeField>, C::ChallengeField) {
    let final_layer_vals = circuit_vals.vals.last().unwrap();
    let mut rz0 = transcript
        .generate_field_elements::<C::ChallengeField>(final_layer_vals.len().trailing_zeros() as usize);
    let r_simd = transcript
        .generate_field_elements::<C::ChallengeField>(C::get_field_pack_size().trailing_zeros() as usize);
    let output_claim = MultiLinearPoly::evaluate_with_buffer(final_layer_vals, &rz0, &mut sp.v_evals);
    let output_claim = MultiLinearPoly::<C::ChallengeField>::evaluate_with_buffer(
        &output_claim.unpack(),
        &r_simd,
        &mut sp.eq_evals_at_r_simd,
    );

    // The second challenge from the previous layer
    let mut rz1;

    // r_relays is generated by the sumcheck_scatter function
    // where r_relays[i] = [(input_layer_id, r)] is the relay claims produced by layer i
    let mut r_relays;

    // r_relays_all keeps track of all the r_relays generated in the previous layers
    // where r_relays_all[i] = [(output_layer_id, r)] is all the relay claims for layer i
    let mut r_relays_all = vec![vec![]; circuit.layers.len()];
    let mut input_claim = C::ChallengeField::zero();

    for i in (1..circuit.layers.len()).rev() {
        let layer = &circuit.layers[i];
        (rz0, rz1, r_relays) = sumcheck_prove_scatter_layer(
            layer,
            &rz0,
            &r_simd,
            connections,
            circuit_vals,
            transcript,
            sp,
        );
        for (input_layer, r) in r_relays {
            r_relays_all[input_layer].push((i, r));
        }

        let layer_next = &circuit.layers[i - 1];
        (rz0, input_claim) = sumcheck_prove_gather_layer(
            layer_next,
            &rz0,
            &rz1,
            &r_relays_all[i - 1],
            connections,
            circuit_vals,
            transcript,
            sp,
        );
    }

    let input_challenge = rz0;

    (output_claim, input_challenge, input_claim)
}
