use std::{
    fs,
    io::Cursor,
    process::exit,
    sync::{Arc, Mutex},
};

use arith::{Field, FieldSerde, FieldSerdeError};
use circuit::Circuit;
use config::{
    Config, GKRConfig, PolynomialCommitmentType, SENTINEL_BN254, SENTINEL_GF2, SENTINEL_M31,
};
use config_macros::declare_gkr_config;
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};

use poly_commit::{expander_pcs_init_testing_only, raw::RawExpanderGKR};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use transcript::{BytesHashTranscript, FieldHashTranscript, MIMCHasher, SHA256hasher};

use log::info;
use transcript::Proof;
use warp::{http::StatusCode, reply, Filter};

#[allow(unused_imports)] // The FiatShamirHashType import is used in the macro expansion
use config::FiatShamirHashType;
#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;

pub fn dump_proof_and_claimed_v<F: Field>(
    proof: &Proof,
    claimed_v: &F,
) -> Result<Vec<u8>, FieldSerdeError> {
    let mut bytes = Vec::new();

    proof.serialize_into(&mut bytes)?;
    claimed_v.serialize_into(&mut bytes)?;

    Ok(bytes)
}

pub fn load_proof_and_claimed_v<F: Field>(bytes: &[u8]) -> Result<(Proof, F), FieldSerdeError> {
    let mut cursor = Cursor::new(bytes);

    let proof = Proof::deserialize_from(&mut cursor)?;
    let claimed_v = F::deserialize_from(&mut cursor)?;

    Ok((proof, claimed_v))
}

pub fn detect_field_type_from_circuit_file(circuit_file: &str) -> FieldType {
    // read last 32 byte of sentinel field element to determine field type
    let bytes = fs::read(circuit_file).expect("Unable to read circuit file.");
    let field_bytes = &bytes[8..8 + 32];
    match field_bytes.try_into().unwrap() {
        SENTINEL_M31 => FieldType::M31,
        SENTINEL_BN254 => FieldType::BN254,
        SENTINEL_GF2 => FieldType::GF2,
        _ => {
            println!("Unknown field type. Field byte value: {:?}", field_bytes);
            exit(1);
        }
    }
}

const PCS_TESTING_SEED_U64: u64 = 114514;

pub fn prove<Cfg: GKRConfig>(
    circuit: &mut Circuit<Cfg::FieldConfig>,
    config: &Config<Cfg>,
) -> (
    <<Cfg as GKRConfig>::FieldConfig as GKRFieldConfig>::ChallengeField,
    Proof,
) {
    let mut prover = crate::Prover::new(config);
    prover.prepare_mem(circuit);
    // TODO: Read PCS setup from files

    let mut rng = ChaCha12Rng::seed_from_u64(PCS_TESTING_SEED_U64);

    let (pcs_params, pcs_proving_key, _pcs_verification_key, mut pcs_scratch) =
        expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::Transcript, Cfg::PCS>(
            circuit.log_input_size(),
            &config.mpi_config,
            &mut rng,
        );

    prover.prove(circuit, &pcs_params, &pcs_proving_key, &mut pcs_scratch)
}

pub fn verify<Cfg: GKRConfig>(
    circuit: &mut Circuit<Cfg::FieldConfig>,
    config: &Config<Cfg>,
    proof: &Proof,
    claimed_v: &<<Cfg as GKRConfig>::FieldConfig as GKRFieldConfig>::ChallengeField,
) -> bool {
    // TODO: Read PCS setup from files
    let mut rng = ChaCha12Rng::seed_from_u64(PCS_TESTING_SEED_U64);

    let (pcs_params, _pcs_proving_key, pcs_verification_key, mut _pcs_scratch) =
        expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::Transcript, Cfg::PCS>(
            circuit.log_input_size(),
            &config.mpi_config,
            &mut rng,
        );
    let verifier = crate::Verifier::new(config);
    let public_input = circuit.public_input.clone();
    verifier.verify(
        circuit,
        &public_input,
        claimed_v,
        &pcs_params,
        &pcs_verification_key,
        proof,
    )
}

pub async fn run_command<'a, Cfg: GKRConfig>(
    command: &str,
    circuit_file: &str,
    config: Config<Cfg>,
    args: &[String],
) {
    match command {
        "prove" => {
            let witness_file = &args[3];
            let output_file = &args[4];
            let mut circuit = Circuit::<Cfg::FieldConfig>::load_circuit(circuit_file);
            circuit.load_witness_file(witness_file);

            let (claimed_v, proof) = prove(&mut circuit, &config);

            if config.mpi_config.is_root() {
                let bytes = dump_proof_and_claimed_v(&proof, &claimed_v)
                    .expect("Unable to serialize proof.");
                fs::write(output_file, bytes).expect("Unable to write proof to file.");
            }
        }
        "verify" => {
            let witness_file = &args[3];
            let output_file = &args[4];
            let mut circuit = Circuit::<Cfg::FieldConfig>::load_circuit(circuit_file);
            circuit.load_witness_file(witness_file);

            // Repeating the same public input for mpi_size times
            // TODO: Fix this, use real input
            if args.len() > 5 {
                let mpi_size = args[5].parse::<i32>().unwrap();
                let n_public_input_per_mpi = circuit.public_input.len();
                for _ in 1..mpi_size {
                    circuit
                        .public_input
                        .append(&mut circuit.public_input[..n_public_input_per_mpi].to_owned());
                }
            }
            let bytes = fs::read(output_file).expect("Unable to read proof from file.");
            let (proof, claimed_v) =
                load_proof_and_claimed_v(&bytes).expect("Unable to deserialize proof.");

            assert!(verify(&mut circuit, &config, &proof, &claimed_v));

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
            let circuit = Circuit::<Cfg::FieldConfig>::load_circuit(circuit_file);
            let mut prover = crate::Prover::new(&config);
            prover.prepare_mem(&circuit);
            let verifier = crate::Verifier::new(&config);

            // TODO: Read PCS  setup from files
            let mut rng = ChaCha12Rng::seed_from_u64(PCS_TESTING_SEED_U64);
            let (pcs_params, pcs_proving_key, pcs_verification_key, pcs_scratch) =
                expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::Transcript, Cfg::PCS>(
                    circuit.log_input_size(),
                    &config.mpi_config,
                    &mut rng,
                );

            let circuit = Arc::new(Mutex::new(circuit));
            let circuit_clone_for_verifier = circuit.clone();
            let prover = Arc::new(Mutex::new(prover));
            let verifier = Arc::new(Mutex::new(verifier));
            let pcs_params = Arc::new(Mutex::new(pcs_params));
            let pcs_params_clone_for_verifier = pcs_params.clone();
            let pcs_proving_key = Arc::new(Mutex::new(pcs_proving_key));
            let pcs_verification_key = Arc::new(Mutex::new(pcs_verification_key));
            let pcs_scratch = Arc::new(Mutex::new(pcs_scratch));

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
                        let witness_bytes: Vec<u8> = bytes.to_vec();
                        let mut circuit = circuit.lock().unwrap();
                        let mut prover = prover.lock().unwrap();
                        let pcs_params = pcs_params.lock().unwrap();
                        let pcs_proving_key = pcs_proving_key.lock().unwrap();
                        let mut pcs_scratch = pcs_scratch.lock().unwrap();

                        circuit.load_witness_bytes(&witness_bytes, true);
                        let (claimed_v, proof) = prover.prove(
                            &mut circuit,
                            &pcs_params,
                            &pcs_proving_key,
                            &mut pcs_scratch,
                        );
                        reply::with_status(
                            dump_proof_and_claimed_v(&proof, &claimed_v).unwrap(),
                            StatusCode::OK,
                        )
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
                        let pcs_verification_key = pcs_verification_key.lock().unwrap();
                        circuit.load_witness_bytes(witness_bytes, true);
                        let public_input = circuit.public_input.clone();
                        let (proof, claimed_v) = load_proof_and_claimed_v(proof_bytes).unwrap();
                        if verifier.verify(
                            &mut circuit,
                            &public_input,
                            &claimed_v,
                            &pcs_params_clone_for_verifier.lock().unwrap(),
                            &pcs_verification_key,
                            &proof,
                        ) {
                            "success".to_string()
                        } else {
                            "failure".to_string()
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

declare_gkr_config!(
    pub M31ExtConfigSha2,
    FieldType::M31,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);
declare_gkr_config!(
    pub BN254ConfigMIMC5,
    FieldType::BN254,
    FiatShamirHashType::MIMC5,
    PolynomialCommitmentType::Raw
);
declare_gkr_config!(
    pub GF2ExtConfigSha2,
    FieldType::GF2,
    FiatShamirHashType::SHA256,
    PolynomialCommitmentType::Raw
);
