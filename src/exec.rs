use std::{fs, vec};

use arith::{Field, FieldSerde, VectorizedM31};
use expander_rs::{Circuit, Config, Proof, Prover, Verifier};

type F = VectorizedM31;

fn dump_proof_and_claimed_v(proof: &Proof, claimed_v: &[F]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let proof_len = proof.bytes.len();
    let claimed_v_len = claimed_v.len();
    bytes.extend_from_slice(&proof_len.to_le_bytes());
    bytes.extend_from_slice(&proof.bytes);
    bytes.extend_from_slice(&claimed_v_len.to_le_bytes());
    for v in claimed_v.iter() {
        let mut buffer = vec![0u8; F::SIZE];
        v.serialize_into(&mut buffer);
        bytes.extend_from_slice(&buffer);
    }
    bytes
}

fn load_proof_and_claimed_v(bytes: &[u8]) -> (Proof, Vec<F>) {
    let mut offset = 0;
    let proof_len = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
    offset += 8;
    let proof_bytes = bytes[offset..offset + proof_len].to_vec();
    offset += proof_len;
    let claimed_v_len = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
    offset += 8;
    let mut claimed_v = Vec::new();
    for _ in 0..claimed_v_len {
        let mut buffer = [0u8; F::SIZE];
        buffer.copy_from_slice(&bytes[offset..offset + F::SIZE]);
        offset += F::SIZE;
        claimed_v.push(F::deserialize_from(&buffer));
    }
    let mut proof = Proof::default();
    proof.bytes = proof_bytes;
    (proof, claimed_v)
}

fn main() {
    // examples:
    // expander-exec prove <input:circuit_file> <input:witness_file> <output:proof>
    // expander-exec verify <input:circuit_file> <input:witness_file> <input:proof>
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 4 {
        println!(
            "Usage: expander-exec prove <input:circuit_file> <input:witness_file> <output:proof>"
        );
        println!(
            "Usage: expander-exec verify <input:circuit_file> <input:witness_file> <input:proof>"
        );
        return;
    }
    let command = &args[1];
    let circuit_file = &args[2];
    let witness_file = &args[3];
    let output_file = &args[4];
    match command.as_str() {
        "prove" => {
            let config = Config::new();
            let mut circuit = Circuit::<F>::load_circuit(circuit_file);
            circuit.load_witness(witness_file);
            circuit.evaluate();
            let mut prover = Prover::new(&config);
            prover.prepare_mem(&circuit);
            let (claimed_v, proof) = prover.prove(&circuit);
            let bytes = dump_proof_and_claimed_v(&proof, &claimed_v);
            fs::write(output_file, bytes).expect("Unable to write proof to file.");
        }
        "verify" => {
            let config = Config::new();
            let mut circuit = Circuit::<F>::load_circuit(circuit_file);
            circuit.load_witness(witness_file);
            let bytes = fs::read(output_file).expect("Unable to read proof from file.");
            let (proof, claimed_v) = load_proof_and_claimed_v(&bytes);
            let verifier = Verifier::new(&config);
            assert!(verifier.verify(&circuit, &claimed_v, &proof));
            println!("success")
        }
        _ => {
            println!("Invalid command.");
        }
    }
}
