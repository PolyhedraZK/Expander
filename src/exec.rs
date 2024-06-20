use std::{
    fs,
    sync::{Arc, Mutex},
    vec,
};

use arith::{Field, FieldSerde, VectorizedM31};
use expander_rs::{Circuit, Config, Proof, Prover, Verifier};
use warp::Filter;

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

#[tokio::main]
async fn main() {
    // examples:
    // expander-exec prove <input:circuit_file> <input:witness_file> <output:proof>
    // expander-exec verify <input:circuit_file> <input:witness_file> <input:proof>
    // expander-exec serve <input:circuit_file> <input:host> <input:port>
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 4 {
        println!(
            "Usage: expander-exec prove <input:circuit_file> <input:witness_file> <output:proof>"
        );
        println!(
            "Usage: expander-exec verify <input:circuit_file> <input:witness_file> <input:proof>"
        );
        println!("Usage: expander-exec serve <input:circuit_file> <input:host> <input:port>");
        return;
    }
    let command = &args[1];
    let circuit_file = &args[2];
    match command.as_str() {
        "prove" => {
            let witness_file = &args[3];
            let output_file = &args[4];
            let config = Config::m31_config();
            let mut circuit = Circuit::<F>::load_circuit(circuit_file);
            circuit.load_witness_file(witness_file);
            circuit.evaluate();
            let mut prover = Prover::new(&config);
            prover.prepare_mem(&circuit);
            let (claimed_v, proof) = prover.prove(&circuit);
            let bytes = dump_proof_and_claimed_v(&proof, &claimed_v);
            fs::write(output_file, bytes).expect("Unable to write proof to file.");
        }
        "verify" => {
            let witness_file = &args[3];
            let output_file = &args[4];
            let config = Config::m31_config();
            let mut circuit = Circuit::<F>::load_circuit(circuit_file);
            circuit.load_witness_file(witness_file);
            let bytes = fs::read(output_file).expect("Unable to read proof from file.");
            let (proof, claimed_v) = load_proof_and_claimed_v(&bytes);
            let verifier = Verifier::new(&config);
            assert!(verifier.verify(&circuit, &claimed_v, &proof));
            println!("success");
        }
        "serve" => {
            let host: [u8; 4] = args[3]
                .split('.')
                .map(|s| s.parse().unwrap())
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap();
            let port = args[4].parse().unwrap();
            let config = Config::m31_config();
            let circuit = Circuit::<F>::load_circuit(circuit_file);
            let mut prover = Prover::new(&config);
            prover.prepare_mem(&circuit);
            let verifier = Verifier::new(&config);
            let circuit = Arc::new(Mutex::new(circuit));
            let circuit_clone_for_verifier = circuit.clone();
            let prover = Arc::new(Mutex::new(prover));
            let verifier = Arc::new(Mutex::new(verifier));

            let prove =
                warp::path("prove")
                    .and(warp::body::bytes())
                    .map(move |bytes: bytes::Bytes| {
                        let witness_bytes: Vec<u8> = bytes.to_vec();
                        let mut circuit = circuit.lock().unwrap();
                        let mut prover = prover.lock().unwrap();
                        circuit.load_witness_bytes(&witness_bytes);
                        circuit.evaluate();
                        let (claimed_v, proof) = prover.prove(&circuit);
                        dump_proof_and_claimed_v(&proof, &claimed_v)
                    });
            let verify =
                warp::path("verify")
                    .and(warp::body::bytes())
                    .map(move |bytes: bytes::Bytes| {
                        let witness_and_proof_bytes: Vec<u8> = bytes.to_vec();
                        let length_of_witness_bytes =
                            u64::from_le_bytes(witness_and_proof_bytes[0..8].try_into().unwrap())
                                as usize;
                        let length_of_proof_bytes =
                            u64::from_le_bytes(witness_and_proof_bytes[8..16].try_into().unwrap())
                                as usize;
                        let witness_bytes =
                            &witness_and_proof_bytes[16..16 + length_of_witness_bytes];
                        let proof_bytes = &witness_and_proof_bytes[16 + length_of_witness_bytes
                            ..16 + length_of_witness_bytes + length_of_proof_bytes];

                        let mut circuit = circuit_clone_for_verifier.lock().unwrap();
                        let verifier = verifier.lock().unwrap();
                        circuit.load_witness_bytes(witness_bytes);
                        let (proof, claimed_v) = load_proof_and_claimed_v(proof_bytes);
                        if verifier.verify(&circuit, &claimed_v, &proof) {
                            "success".to_string()
                        } else {
                            "failure".to_string()
                        }
                    });
            warp::serve(warp::post().and(prove.or(verify)))
                .run((host, port))
                .await;
        }
        _ => {
            println!("Invalid command.");
        }
    }
}
