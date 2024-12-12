use arith::Field;
use crosslayer_prototype::{CrossLayerCircuit, CrossLayerConnections, prove_gkr};
use gkr_field_config::{GKRFieldConfig, M31ExtConfig, GF2ExtConfig, BN254Config};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, SHA256hasher, Transcript};

fn test_sumcheck_cross_layered_helper<C: GKRFieldConfig>() {
    let mut transcript = BytesHashTranscript::<C::ChallengeField, SHA256hasher>::new();
    
    let mut rng = rand::thread_rng();
    let n_layers = 1323;
    let layer_size = 512;
    let n_gates_each_layer = 256;

    let circuit = CrossLayerCircuit::<C>::random_for_bench(&mut rng, n_layers, layer_size, n_gates_each_layer);
    
    let inputs = (0..circuit.layers[0].layer_size).map(|_| C::ChallengeField::random_unsafe(&mut rng)).collect::<Vec<_>>();
    let evals = circuit.evaluate(&inputs);
    let connections = CrossLayerConnections::parse_circuit(&circuit);
    let start_time = std::time::Instant::now();
    let mut sp = crosslayer_prototype::CrossLayerProverScratchPad::<C>::new(circuit.layers.len(), circuit.max_num_input_var(), circuit.max_num_output_var(), 1);
    let (_output_claim, input_challenge, input_claim) = prove_gkr(&circuit, &evals, &connections, &mut transcript, &mut sp);
    let stop_time = std::time::Instant::now();
    let duration = stop_time.duration_since(start_time);
    println!("Time elapsed {} ms", duration.as_millis());

    assert!(
        MultiLinearPoly::evaluate_with_buffer(&evals.vals[0], &input_challenge, &mut sp.v_evals) == input_claim,
    );
}

fn main() {
    test_sumcheck_cross_layered_helper::<GF2ExtConfig>(); 
}