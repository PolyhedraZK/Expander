use arith::Field;
use gkr_field_config::GKRFieldConfig;
use polynomials::MultiLinearPoly;
use transcript::Transcript;

use crate::{circuit, scratchpad, CrossLayerCircuit};
use crate::sumcheck::{sumcheck_prove_scatter_layer, sumcheck_prove_gather_layer};

pub fn prove_gkr<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    circuit: &CrossLayerCircuit<C>,
    circuit_vals: &circuit::CrossLayerCircuitEvals<C>,
    connections: &circuit::CrossLayerConnections,
    transcript: &mut T,
    sp: &mut scratchpad::CrossLayerProverScratchPad<C>,
) -> (C::ChallengeField, Vec<C::ChallengeField>, C::ChallengeField) {

    let final_layer_vals = circuit_vals.vals.last().unwrap();
    let mut rz0 = transcript.generate_challenge_field_elements(final_layer_vals.len().trailing_zeros() as usize);
    let output_claim = MultiLinearPoly::evaluate_with_buffer(final_layer_vals, &rz0, &mut sp.v_evals);

    // index: output_layer, input_layer, random challenge
    let mut rz1;
    let mut r_relays;
    let mut relay_claims = vec![vec![]; circuit.layers.len()];
    let mut input_claim = C::ChallengeField::zero();

    for i in (1..circuit.layers.len()).rev() {
        let layer = &circuit.layers[i];
        (rz0, rz1, r_relays) = sumcheck_prove_scatter_layer(layer, &rz0, &relay_claims[i], connections, circuit_vals, transcript, sp);
        for (input_layer, r) in r_relays {
            relay_claims[input_layer].push((i, r));
        }

        let layer_next = &circuit.layers[i - 1];
        (rz0, input_claim) = sumcheck_prove_gather_layer(layer_next, &rz0, &rz1, &relay_claims[i - 1], connections, circuit_vals, transcript, sp);
    }

    let input_challenge = rz0;

    (output_claim, input_challenge, input_claim)
}