use arith::Field;
use crosslayer_prototype::{CrossLayerCircuit, CrossLayerConnections, prove_gkr};
use gkr_field_config::{GKRFieldConfig, M31ExtConfig, GF2ExtConfig, BN254Config};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, SHA256hasher, Transcript};

fn test_sumcheck_cross_layered_helper<C: GKRFieldConfig>() {
    let mut transcript = BytesHashTranscript::<C::ChallengeField, SHA256hasher>::new();
    
    let mut rng = rand::thread_rng();
    let n_layers = 3;
    let circuit = CrossLayerCircuit::<C>::random_for_testing(&mut rng, n_layers);
    
    let inputs = (0..circuit.layers[0].layer_size).map(|_| C::ChallengeField::random_unsafe(&mut rng)).collect::<Vec<_>>();
    let evals = circuit.evaluate(&inputs);
    let connections = CrossLayerConnections::parse_circuit(&circuit);
    let mut sp = crosslayer_prototype::CrossLayerProverScratchPad::<C>::new(circuit.max_num_input_var(), circuit.max_num_output_var(), 1);

    let (_output_claim, input_challenge, input_claim) = prove_gkr(&circuit, &evals, &connections, &mut transcript, &mut sp);

    assert!(
        MultiLinearPoly::evaluate_with_buffer(&evals.vals[0], &input_challenge, &mut sp.v_evals) == input_claim,
    );
}

#[test]
fn test_sumcheck_cross_layered() {
    test_sumcheck_cross_layered_helper::<M31ExtConfig>();
    test_sumcheck_cross_layered_helper::<GF2ExtConfig>();
    test_sumcheck_cross_layered_helper::<BN254Config>();
}