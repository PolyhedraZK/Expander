use std::io::Write;

use circuit::Circuit;
use gkr::{
    Prover,
    utils::{KECCAK_M31_CIRCUIT, KECCAK_M31_WITNESS},
};
use gkr_engine::GKRScheme;
use gkr_engine::{GKREngine, Goldilocksx1Config, MPIConfig, MPIEngine, SharedMemory};
use gkr_hashers::SHA256hasher;
use poly_commit::RawExpanderGKR;
use poly_commit::expander_pcs_init_testing_only;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;

struct Goldilocksx1Sha2RawCudaDev;

impl GKREngine for Goldilocksx1Sha2RawCudaDev {
    type FieldConfig = Goldilocksx1Config;
    type MPIConfig = MPIConfig;
    type TranscriptConfig = BytesHashTranscript<SHA256hasher>;
    type PCSConfig = RawExpanderGKR<Goldilocksx1Config>;
    const SCHEME: GKRScheme = GKRScheme::Vanilla;
    const CUDA_DEV: bool = true;
}

fn main() {
    proof_gen_x16::<Goldilocksx1Sha2RawCudaDev>();
}

pub fn proof_gen_x16<C: GKREngine>() {
    let mpi_config = MPIConfig::prover_new();

    // load circuit
    let (mut circuit, mut window) =
        Circuit::<C::FieldConfig>::prover_load_circuit::<C>(KECCAK_M31_CIRCUIT, &mpi_config);

    let witness_path = KECCAK_M31_WITNESS;

    let proof_file_name = "data/proof_m31x1_cuda_dev.txt";

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

    circuit.discard_control_of_shared_mem();
    mpi_config.free_shared_mem(&mut window);
}
