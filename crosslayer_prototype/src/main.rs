use arith::Field;
use crosslayer_prototype::{prove_gkr, CrossLayerCircuit, CrossLayerConnections, CrossLayerRecursiveCircuit};
use gkr_field_config::{GKRFieldConfig, M31ExtConfig, GF2ExtConfig, BN254Config};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, SHA256hasher, Transcript};

fn test_sumcheck_cross_layered_helper<C: GKRFieldConfig>() {
    let mut transcript = BytesHashTranscript::<C::ChallengeField, SHA256hasher>::new();
    
    let mut rng = rand::thread_rng();

    let circuit = CrossLayerRecursiveCircuit::<C>::load("./data/sha256_circuit_gf2.txt").unwrap().flatten();
    circuit.print_stats();    

    let inputs = (0..circuit.layers[0].layer_size).map(|_| C::ChallengeField::random_unsafe(&mut rng)).collect::<Vec<_>>();
    let connections = CrossLayerConnections::parse_circuit(&circuit);

    let start_time = std::time::Instant::now();
    let evals = circuit.evaluate(&inputs);
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