use std::{
    fs,
    io::Cursor,
    process::exit,
    sync::{Arc, Mutex}, thread,
};

use arith::{Field, FieldSerde};
use expander_rs::{
    BN254ConfigSha2, Circuit, Config, FieldType, GKRConfig, GKRScheme, M31ExtConfigSha2, Proof,
    Prover, Verifier, SENTINEL_BN254, SENTINEL_M31,
};
use log::{debug, info};
use tokio::task;
use warp::{http::StatusCode, reply, Filter};
const THREAD: usize = 1;

fn dump_proof_and_claimed_v<F: Field + FieldSerde>(proof: &Proof, claimed_v: &F) -> Vec<u8> {
    let mut bytes = Vec::new();

    proof.serialize_into(&mut bytes).unwrap(); // TODO: error propagation
    claimed_v.serialize_into(&mut bytes).unwrap(); // TODO: error propagation

    bytes
}

fn load_proof_and_claimed_v<F: Field + FieldSerde>(bytes: &[u8]) -> (Proof, F) {
    let mut cursor = Cursor::new(bytes);

    let proof = Proof::deserialize_from(&mut cursor).unwrap(); // TODO: error propagation
    let claimed_v = F::deserialize_from(&mut cursor).unwrap(); // TODO: error propagation

    (proof, claimed_v)
}

fn detect_field_type_from_circuit_file(circuit_file: &str) -> FieldType {
    // read last 32 byte of sentinel field element to determine field type
    let bytes = fs::read(circuit_file).expect("Unable to read circuit file.");
    let field_bytes = &bytes[8..8 + 32];
    match field_bytes.try_into().unwrap() {
        SENTINEL_M31 => FieldType::M31,
        SENTINEL_BN254 => FieldType::BN254,
        _ => {
            println!("Unknown field type. Field byte value: {:?}", field_bytes);
            exit(1);
        }
    }
}

async fn run_command<C: GKRConfig>(
    command: &str,
    circuit_file: &str,
    config: Config<C>,
    args: &[String],
) {
    match command {
        "prove" => {
            let witness_file = &args[3];
            let output_file = &args[4];
            let mut circuit = Circuit::<C>::load_circuit(circuit_file);
            println!("Circuit loaded.");
            let start_time = std::time::Instant::now();
            circuit.load_witness_file(witness_file);
            println!("Witness loaded. {:?}", start_time.elapsed());
            circuit.evaluate();
            println!("evaluate circuit. {:?}", start_time.elapsed());
            let mut prover = Prover::new(&config);
            println!("Prover created.{:?}", start_time.elapsed());
            prover.prepare_mem(&circuit);
            println!("Prover prepared.{:?}", start_time.elapsed());
            let (claimed_v, proof) = prover.prove(&mut circuit);
            let bytes = dump_proof_and_claimed_v(&proof, &claimed_v);
            let elapsed = start_time.elapsed();
            println!("Proof finished: {:.2?}", elapsed);
            fs::write(output_file, bytes).expect("Unable to write proof to file.");
            let elapsed2 = start_time.elapsed();
            println!("Proof File saved: {:.2?}", elapsed2);
        }
        "verify" => {
            let witness_file = &args[3];
            let output_file = &args[4];
            let mut circuit = Circuit::<C>::load_circuit(circuit_file);
            circuit.load_witness_file(witness_file);
            let bytes = fs::read(output_file).expect("Unable to read proof from file.");
            let (proof, claimed_v) = load_proof_and_claimed_v(&bytes);
            let verifier = Verifier::new(&config);
            assert!(verifier.verify(&mut circuit, &claimed_v, &proof));
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
            let circuit = Circuit::<C>::load_circuit(circuit_file);
            let mut prover = Prover::new(&config);
            prover.prepare_mem(&circuit);
            let verifier = Verifier::new(&config);
            let circuit = Arc::new(Mutex::new(circuit));
            let circuit_clone_for_verifier = circuit.clone();
            let prover = Arc::new(Mutex::new(prover));
            let verifier = Arc::new(Mutex::new(verifier));
            let ready_time = chrono::offset::Utc::now();
            let ready = warp::path("ready").map(move || {
                info!("Received ready request.");
                reply::with_status(format!("Ready since {:?}", ready_time), StatusCode::OK)
            });
            let prove =
                warp::path("prove")
                    .and(warp::body::bytes())
                    .map(move |bytes: bytes::Bytes| {
                        info!("Received prove request.");
                        println!("Received prove request.");
                        let start_time = std::time::Instant::now();
                        let witness_bytes: Vec<u8> = bytes.to_vec();
                        let mut circuit = circuit.lock().unwrap();
                        circuit.set_random_input_for_test();
                        println!("load witness. {:?}", start_time.elapsed());
                        circuit.evaluate();
                        println!("evaluate circuit. {:?}", start_time.elapsed());
                        let mut prover = prover.lock().unwrap();
                        let (claimed_v, proof) = prover.prove(&mut circuit);
                        let elapsed = start_time.elapsed();
                        println!("Proof finished: {:.2?}", elapsed);
                        reply::with_status(
                            dump_proof_and_claimed_v(&proof, &claimed_v),
                            StatusCode::OK,
                        )
                        // if circuit.load_witness_bytes(&witness_bytes).is_err() {
                        //     reply::with_status(vec![], StatusCode::BAD_REQUEST)
                        // } else {
                        //     println!("load witness. {:?}", start_time.elapsed());
                        //     let start_time2 = std::time::Instant::now();
                        //     circuit.evaluate();
                        //     let start_time3 = std::time::Instant::now();
                        //     println!("evaluate circuit. {:?}, from load: {:?}", start_time.elapsed(), start_time2.elapsed());
                        //     prover.prove(&mut circuit);
                        //     println!("Proof finished: {:.2?}, from load: {:?}, from evaluate {:?}",  start_time.elapsed(), start_time2.elapsed(), start_time3.elapsed());
                        //     reply::with_status(vec![], StatusCode::BAD_REQUEST)
                        //     // reply::with_status(
                        //     //     dump_proof_and_claimed_v(&proof, &claimed_v),
                        //     //     StatusCode::OK,
                        //     // )
                        // }
                    });
            let verify =
                warp::path("verify")
                    .and(warp::body::bytes())
                    .map(move |bytes: bytes::Bytes| {
                        info!("Received verify request.");
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
                        if circuit.load_witness_bytes(witness_bytes).is_err() {
                            "failure".to_string()
                        } else {
                            let start_time = std::time::Instant::now();
                            let (proof, claimed_v) = load_proof_and_claimed_v(proof_bytes);
                            println!("load proof. {:?}", start_time.elapsed());
                            if verifier.verify(&mut circuit, &claimed_v, &proof) {
                                println!("verify success. {:?}", start_time.elapsed());
                                "success".to_string()
                            } else {
                                println!("verify failure. {:?}", start_time.elapsed());
                                "failure".to_string()
                            }
                        }
                    });
            warp::serve(
                warp::post()
                    .and(prove.or(verify))
                    .or(warp::get().and(ready)),
            )
            .run((host, port))
            .await;
        }
        _ => {
            println!("Invalid command.");
        }
    }
}

#[tokio::main]
async fn main() {
    // examples:
    // expander-exec prove <input:circuit_file> <input:witness_file> <output:proof>
    // expander-exec verify <input:circuit_file> <input:witness_file> <input:proof>
    // expander-exec serve <input:circuit_file> <input:ip> <input:port>
    env_logger::init();
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
    let field_type = detect_field_type_from_circuit_file(circuit_file);
    debug!("field type: {:?}", field_type);
    println!("field type: {:?}", field_type);
    match field_type {
        FieldType::M31 => {
            println!("inside m31");
            run_command::<M31ExtConfigSha2>(
                command,
                circuit_file,
                Config::<M31ExtConfigSha2>::new(GKRScheme::Vanilla),
                &args,
            )
            .await;
        }
        FieldType::BN254 => {
            run_command::<BN254ConfigSha2>(
                command,
                circuit_file,
                Config::<BN254ConfigSha2>::new(GKRScheme::Vanilla),
                &args,
            )
            .await;
        }
        _ => unreachable!(),
    }
    // let start_time = std::time::Instant::now();
    // match field_type {
    //     FieldType::M31 => {
    //         println!("inside m31");
    //         for _ in 0..THREAD {
    //             let command = command.to_string();
    //             let circuit_file = circuit_file.to_string();
    //             let new_args = args.clone();
    //             let handle = task::spawn({
    //                 async move {
    //                     // bench func
    //                     run_command::<M31ExtConfigSha2>(
    //                         &command,
    //                         &circuit_file,
    //                         Config::<M31ExtConfigSha2>::new(GKRScheme::Vanilla),
    //                         &new_args,
    //                     ).await;
    //                 }
    //             });
    //             handles.push(handle);
    //         }
    //         for handle in handles {
    //             handle.await.unwrap(); 
    //         }
    //     }
    //     FieldType::BN254 => {
    //         println!("inside bn254");
    //         for _ in 0..THREAD {
    //             let command = command.to_string();
    //             let circuit_file = circuit_file.to_string();
    //             let new_args = args.clone();
    //             let handle = task::spawn({
    //                 async move {
    //                     // bench func
    //                     run_command::<BN254ConfigSha2>(
    //                         &command,
    //                         &circuit_file,
    //                         Config::<BN254ConfigSha2>::new(GKRScheme::Vanilla),
    //                         &new_args,
    //                     ).await;
    //                 }
    //             });
    //             handles.push(handle);
    //         }
    //         for handle in handles {
    //             handle.await.unwrap(); 
    //         }
    //     }
    //     _ => unreachable!(),
    // }
    // let elapsed = start_time.elapsed();
    // println!("Elapsed: {:.2?}", elapsed);
}
