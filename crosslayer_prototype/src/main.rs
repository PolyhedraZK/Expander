use arith::Field;
use crosslayer_prototype::{prove_gkr, CrossLayerConnections, CrossLayerRecursiveCircuit};
use gkr_field_config::{GF2ExtConfig, GKRFieldConfig};
use transcript::{BytesHashTranscript, SHA256hasher, Transcript};

fn test_sumcheck_cross_layered_helper<C: GKRFieldConfig>() {
    let mut transcript = BytesHashTranscript::<C::ChallengeField, SHA256hasher>::new();

    let mut rng = rand::thread_rng();

    let circuit = CrossLayerRecursiveCircuit::<C>::load("./data/sha256_circuit_gf2.txt")
        .unwrap()
        .flatten();
    circuit.print_stats();

    let inputs = (0..circuit.layers[0].layer_size)
        .map(|_| C::SimdCircuitField::random_unsafe(&mut rng))
        .collect::<Vec<_>>();
    let connections = CrossLayerConnections::parse_circuit(&circuit);

    let start_time = std::time::Instant::now();
    let evals = circuit.evaluate(&inputs);
    let mut sp = crosslayer_prototype::CrossLayerProverScratchPad::<C>::new(
        circuit.layers.len(),
        circuit.max_num_input_var(),
        circuit.max_num_output_var(),
        1,
    );
    let (_output_claim, _input_challenge, _input_claim) =
        prove_gkr(&circuit, &evals, &connections, &mut transcript, &mut sp);
    let stop_time = std::time::Instant::now();
    let duration = stop_time.duration_since(start_time);
    println!("Time elapsed {} ms", duration.as_millis());
}

fn main() {
    test_sumcheck_cross_layered_helper::<GF2ExtConfig>();
}
