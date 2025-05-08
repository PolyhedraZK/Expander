use std::io::Write;

use circuit::Circuit;
use gkr::Prover;
use gkr_engine::{GKREngine, MPIConfig, MPIEngine};
use gkr_engine::{GKRScheme, Goldilocksx8Config};
use gkr_hashers::SHA256hasher;
use poly_commit::RawExpanderGKR;
use poly_commit::expander_pcs_init_testing_only;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;

struct Goldilocksx8Sha2RawCudaDev;

// fibonacci like circuits with both add and mul gates
pub const CIRCUIT_DIR: &str = "data/circuit_fib_goldilocks.txt";
pub const WITNESS_DIR: &str = "data/witness_fib_goldilocks.txt";

// // keccak circuits
// pub const CIRCUIT_DIR: &str = "data/circuit_goldilocks.txt";
// pub const WITNESS_DIR: &str = "data/witness_goldilocks.txt";

impl GKREngine for Goldilocksx8Sha2RawCudaDev {
    type FieldConfig = Goldilocksx8Config;
    type MPIConfig = MPIConfig;
    type TranscriptConfig = BytesHashTranscript<SHA256hasher>;
    type PCSConfig = RawExpanderGKR<Goldilocksx8Config>;
    const SCHEME: GKRScheme = GKRScheme::Vanilla;
    const CUDA_DEV: bool = true;
}

fn main() {
    proof_gen_x1::<Goldilocksx8Sha2RawCudaDev>();
}

pub fn proof_gen_x1<C: GKREngine>() {
    let mpi_config = MPIConfig::prover_new();

    // load circuit
    let mut circuit =
        Circuit::<C::FieldConfig>::single_thread_prover_load_circuit::<C>(CIRCUIT_DIR);

    let witness_path = WITNESS_DIR;

    let proof_file_name = "data/proof_goldilocksx8_cuda_dev.txt";

    circuit.load_witness_allow_padding_testing_only(witness_path, &mpi_config);

    circuit.evaluate();

    let (pcs_params, pcs_proving_key, _pcs_verification_key, pcs_scratch) =
        expander_pcs_init_testing_only::<C::FieldConfig, C::PCSConfig>(
            circuit.log_input_size(),
            &mpi_config,
        );

    // generate the proof
    let mut local_circuit = circuit.clone();
    let pcs_params = pcs_params.clone();
    let pcs_proving_key = pcs_proving_key.clone();
    let mut pcs_scratch = pcs_scratch.clone();
    let mut prover = Prover::<C>::new(mpi_config.clone());
    prover.prepare_mem(&local_circuit);

    let (claim, proof) = prover.prove(
        &mut local_circuit,
        &pcs_params,
        &pcs_proving_key,
        &mut pcs_scratch,
    );
    let mut buf = Vec::new();
    claim.serialize_into(&mut buf).unwrap();
    proof.serialize_into(&mut buf).unwrap();

    if mpi_config.is_root() {
        println!("proof {:?}", buf);
        println!("Proof size: {}", buf.len());
        let mut file = std::fs::File::create(proof_file_name).unwrap();
        file.write_all(buf.as_ref()).expect("Unable to write data");
        println!("{} generated", proof_file_name);
    }
}
