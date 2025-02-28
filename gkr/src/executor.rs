use std::{
    fs,
    io::Cursor,
    process::exit,
    sync::{Arc, Mutex},
};

use arith::{Field, FieldSerde, FieldSerdeError};
use circuit::Circuit;
use clap::{Parser, Subcommand};
use config::{Config, GKRConfig, SENTINEL_BN254, SENTINEL_GF2, SENTINEL_M31};
use gkr_field_config::FieldType;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use poly_commit::expander_pcs_init_testing_only;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use log::info;
use transcript::Proof;
use warp::{http::StatusCode, reply, Filter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct ExpanderExecArgs {
    /// Fiat-Shamir Hash: SHA256, or Poseidon, or MiMC5
    #[arg(short, long, default_value = "SHA256")]
    pub fiat_shamir_hash: String,

    /// Polynomial Commitment Scheme: Raw, or Orion
    #[arg(short, long, default_value = "Raw")]
    pub poly_commitment_scheme: String,

    /// Circuit File Path
    #[arg(short, long)]
    pub circuit_file: String,

    /// Prove, Verify, or Serve subcommands
    #[clap(subcommand)]
    pub subcommands: ExpanderExecSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ExpanderExecSubCommand {
    Prove {
        /// Witness File Path
        #[arg(short, long)]
        witness_file: String,

        /// Output Proof Path
        #[arg(short, long)]
        output_proof_file: String,
    },
    Verify {
        /// Witness File Path
        #[arg(short, long)]
        witness_file: String,

        /// Output Proof Path
        #[arg(short, long)]
        input_proof_file: String,

        /// MPI size
        #[arg(short, long, default_value_t = 1)]
        mpi_size: u32,
    },
    Serve {
        /// IP host
        #[arg(short, long)]
        host_ip: String,

        /// IP Port
        #[arg(short, long)]
        port: u16,
    },
}

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

pub async fn run_command<'a, Cfg: GKRConfig>(command: &ExpanderExecArgs, mut config: Config<Cfg>) {
    let subcommands = command.subcommands.clone();

    match subcommands {
        ExpanderExecSubCommand::Prove {
            witness_file,
            output_proof_file,
        } => {
            let mut circuit =
                Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(&command.circuit_file);
            circuit.prover_load_witness_file(&witness_file, &config.mpi_config);
            let (claimed_v, proof) = prove(&mut circuit, &config);

            if config.mpi_config.is_root() {
                let bytes = dump_proof_and_claimed_v(&proof, &claimed_v)
                    .expect("Unable to serialize proof.");
                fs::write(output_proof_file, bytes).expect("Unable to write proof to file.");
            }
        }
        ExpanderExecSubCommand::Verify {
            witness_file,
            input_proof_file,
            mpi_size,
        } => {
            assert!(
                config.mpi_config.world_size() == 1,
                "Do not run verifier with mpiexec."
            );
            config.mpi_config.world_size = mpi_size as i32;

            let mut circuit =
                Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(&command.circuit_file);
            circuit.verifier_load_witness_file(&witness_file, &config.mpi_config);

            let bytes = fs::read(&input_proof_file).expect("Unable to read proof from file.");
            let (proof, claimed_v) =
                load_proof_and_claimed_v(&bytes).expect("Unable to deserialize proof.");

            assert!(verify(&mut circuit, &config, &proof, &claimed_v));

            println!("success");
        }
        ExpanderExecSubCommand::Serve { host_ip, port } => {
            assert!(
                config.mpi_config.world_size() == 1,
                "Serve mode is not compatible with mpi for now."
            );
            let host: [u8; 4] = host_ip
                .split('.')
                .map(|s| s.parse().unwrap())
                .collect::<Vec<u8>>()
                .try_into()
                .unwrap();
            let circuit = Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(&command.circuit_file);
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

                        circuit.load_witness_bytes(&witness_bytes, &MPIConfig::new(), true, true);
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
                        circuit.load_witness_bytes(witness_bytes, &MPIConfig::new(), false, true);
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
    }
}
