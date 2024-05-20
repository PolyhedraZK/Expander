use expander_rs::{
    Circuit, CircuitLayer, Config, GateAdd, GateMul, Prover, VectorizedM31, Verifier, M31,
};

const FILENAME_MUL: &str = "data/ExtractedCircuitMul.txt";
const FILENAME_ADD: &str = "data/ExtractedCircuitAdd.txt";

type FPrimitive = M31;
type F = VectorizedM31;

#[test]
fn test_gkr_correctness() {
    let config = Config::new();
    println!("Config created.");
    // let mut circuit = Circuit::load_extracted_gates(FILENAME_MUL, FILENAME_ADD);
    // circuit.layers = circuit.layers[6..8].to_vec(); // [0, 1]

    let mut circuit = Circuit::default();
    let mut l0 = CircuitLayer::default();
    l0.input_var_num = 2;
    l0.output_var_num = 2;
    l0.add.push(GateAdd {
        i_ids: [0],
        o_id: 0,
        coef: FPrimitive::from(1),
    });
    l0.add.push(GateAdd {
        i_ids: [0],
        o_id: 1,
        coef: FPrimitive::from(1),
    });
    l0.add.push(GateAdd {
        i_ids: [1],
        o_id: 1,
        coef: FPrimitive::from(1),
    });
    l0.mul.push(GateMul {
        i_ids: [0, 2],
        o_id: 2,
        coef: FPrimitive::from(1),
    });
    circuit.layers.push(l0.clone());
    circuit.set_random_bool_input();

    println!("Circuit loaded.");
    circuit.set_random_bool_input();
    circuit.evaluate();
    for i in 0..circuit.layers.len() {
        println!("Layer {}:", i);
        println!("Input: {:?}", circuit.layers[i].input_vals.evals);
    }
    println!("Circuit evaluated.");
    let mut prover = Prover::new(&config);
    prover.prepare_mem(&circuit);
    let (claimed_v, proof) = prover.prove(&circuit);
    println!("Proof generated.");
    // Verify
    let verifier = Verifier::new(&config);
    println!("Verifier created.");
    assert!(verifier.verify(&circuit, &claimed_v, &proof));
    println!("Correct proof verified.");
    // let mut bad_proof = proof.clone();
    // *bad_proof.bytes.last_mut().unwrap() += 1;
    // assert!(!verifier.verify(&circuit, &claimed_v, &bad_proof));
    // println!("Bad proof rejected.");
}
