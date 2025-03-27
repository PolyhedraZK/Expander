use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
};

use circuit::Circuit;
use clap::Parser;

use gkr_engine::{
    ExpanderPCS, FieldEngine, FieldType, GKREngine, MPIConfig, MPIEngine, PolynomialCommitmentType,
    StructuredReferenceString,
};
use poly_commit::expander_pcs_init_testing_only;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use gkr::{
    utils::{
        KECCAK_BN254_CIRCUIT, KECCAK_BN254_WITNESS, KECCAK_GF2_CIRCUIT, KECCAK_GF2_WITNESS,
        KECCAK_GOLDILOCKS_CIRCUIT, KECCAK_GOLDILOCKS_WITNESS, KECCAK_M31_CIRCUIT,
        KECCAK_M31_WITNESS, POSEIDON_M31_CIRCUIT, POSEIDON_M31_WITNESS,
    },
    BN254ConfigMIMC5KZG, BN254ConfigSha2Hyrax, BN254ConfigSha2Raw, GF2ExtConfigSha2Orion,
    GF2ExtConfigSha2Raw, GoldilocksExtConfigSha2Raw, M31ExtConfigSha2OrionSquare,
    M31ExtConfigSha2OrionVanilla, M31ExtConfigSha2RawSquare, M31ExtConfigSha2RawVanilla, Prover,
};

use serdes::ExpSerde;

/// ...
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Field Identifier: fr, m31ext3, gf2ext128, goldilocks
    #[arg(short, long,default_value_t = String::from("m31ext3"))]
    field: String,

    // circuit: keccak, poseidon
    #[arg(short, long, default_value_t = String::from("keccak"))]
    circuit: String,

    /// Polynomial Commitment Scheme: Raw, Hyrax, Orion, KZG
    #[arg(short, long, default_value_t = String::from("Raw"))]
    pcs: String,

    /// number of repeat
    #[arg(short, long, default_value_t = 1)]
    repeats: usize,

    /// number of thread
    #[arg(short, long, default_value_t = 1)]
    threads: u64,
}

fn main() {
    let args = Args::parse();
    print_info(&args);

    let mpi_config = MPIConfig::prover_new();
    let pcs_type = PolynomialCommitmentType::from_str(&args.pcs).unwrap();

    match args.field.as_str() {
        "m31ext3" => match pcs_type {
            PolynomialCommitmentType::Raw => match args.circuit.as_str() {
                "keccak" => run_benchmark::<M31ExtConfigSha2RawVanilla>(&args, mpi_config.clone()),
                "poseidon" => run_benchmark::<M31ExtConfigSha2RawSquare>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            PolynomialCommitmentType::Orion => match args.circuit.as_str() {
                "keccak" => {
                    run_benchmark::<M31ExtConfigSha2OrionVanilla>(&args, mpi_config.clone())
                }
                "poseidon" => {
                    run_benchmark::<M31ExtConfigSha2OrionSquare>(&args, mpi_config.clone())
                }
                _ => unreachable!(""),
            },
            _ => unreachable!("Unsupported PCS type for M31"),
        },
        "fr" => match pcs_type {
            PolynomialCommitmentType::Raw => match args.circuit.as_str() {
                "keccak" => run_benchmark::<BN254ConfigSha2Raw>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            PolynomialCommitmentType::Hyrax => match args.circuit.as_str() {
                "keccak" => run_benchmark::<BN254ConfigSha2Hyrax>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            PolynomialCommitmentType::KZG => match args.circuit.as_str() {
                "keccak" => run_benchmark::<BN254ConfigMIMC5KZG>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            _ => unreachable!("Unsupported PCS type for BN254"),
        },
        "gf2ext128" => match pcs_type {
            PolynomialCommitmentType::Raw => match args.circuit.as_str() {
                "keccak" => run_benchmark::<GF2ExtConfigSha2Raw>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            PolynomialCommitmentType::Orion => match args.circuit.as_str() {
                "keccak" => run_benchmark::<GF2ExtConfigSha2Orion>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            _ => unreachable!("Unsupported PCS type for GF2"),
        },
        "goldilocks" => match pcs_type {
            PolynomialCommitmentType::Raw => match args.circuit.as_str() {
                "keccak" => run_benchmark::<GoldilocksExtConfigSha2Raw>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            _ => unreachable!("Unsupported PCS type for Goldilocks"),
        },
        _ => unreachable!(),
    };

    MPIConfig::finalize();
}

const PCS_TESTING_SEED_U64: u64 = 114514;

fn run_benchmark<'a, Cfg: GKREngine>(args: &'a Args, mpi_config: MPIConfig)
where
    <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Params: 'static,
    <<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::SRS as StructuredReferenceString>::PKey:
        'static,
    <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::ScratchPad: 'a,
    <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::ScratchPad: 'static,
{
    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();

    let pack_size = <Cfg::FieldConfig as FieldEngine>::get_field_pack_size();

    // load circuit
    let mut circuit_template = match args.circuit.as_str() {
        "keccak" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::GF2 => Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(KECCAK_GF2_CIRCUIT),
            FieldType::M31 => Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(KECCAK_M31_CIRCUIT),
            FieldType::BN254 => {
                Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(KECCAK_BN254_CIRCUIT)
            }
            FieldType::Goldilocks => {
                Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(KECCAK_GOLDILOCKS_CIRCUIT)
            }
        },
        "poseidon" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::M31 => {
                Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(POSEIDON_M31_CIRCUIT)
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    let witness_path = match args.circuit.as_str() {
        "keccak" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::GF2 => KECCAK_GF2_WITNESS,
            FieldType::M31 => KECCAK_M31_WITNESS,
            FieldType::BN254 => KECCAK_BN254_WITNESS,
            FieldType::Goldilocks => KECCAK_GOLDILOCKS_WITNESS,
        },
        "poseidon" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::M31 => POSEIDON_M31_WITNESS,
            _ => unreachable!("not supported"),
        },
        _ => unreachable!(),
    };

    circuit_template.load_witness_allow_padding_testing_only(witness_path, &mpi_config);

    let circuit_copy_size: usize = match (Cfg::FieldConfig::FIELD_TYPE, args.circuit.as_str()) {
        (FieldType::GF2, "keccak") => 1,
        (FieldType::M31, "keccak") => 2,
        (FieldType::BN254, "keccak") => 2,
        (FieldType::M31, "poseidon") => 120,
        (FieldType::Goldilocks, "keccak") => 2,
        _ => unreachable!(),
    };

    let circuits = (0..args.threads)
        .map(|_| {
            let mut c = circuit_template.clone();
            c.evaluate();
            c
        })
        .collect::<Vec<_>>();

    println!("Circuit loaded!");

    let mut rng = ChaCha12Rng::seed_from_u64(PCS_TESTING_SEED_U64);
    let (pcs_params, pcs_proving_key, _pcs_verification_key, pcs_scratch) =
        expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::PCSConfig>(
            circuit_template.log_input_size(),
            &mpi_config,
            &mut rng,
        );

    // calculate the proof size
    {
        let mut local_circuit = circuits[0].clone();
        let pcs_params = pcs_params.clone();
        let pcs_proving_key = pcs_proving_key.clone();
        let mut pcs_scratch = pcs_scratch.clone();
        let mut prover = Prover::<Cfg>::new(mpi_config.clone());
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
        println!("Proof size: {}", buf.len());
    }

    let start_time = std::time::Instant::now();
    let _ = circuits
        .into_iter()
        .enumerate()
        .map(|(i, mut c)| {
            let local_config = mpi_config.clone();
            let partial_proof_cnt = partial_proof_cnts[i].clone();
            let pcs_params = pcs_params.clone();
            let pcs_proving_key = pcs_proving_key.clone();
            let mut pcs_scratch = pcs_scratch.clone();
            thread::spawn(move || {
                // bench func
                let mut prover = Prover::<Cfg>::new(local_config.clone());
                prover.prepare_mem(&c);
                loop {
                    prover.prove(&mut c, &pcs_params, &pcs_proving_key, &mut pcs_scratch);
                    // update cnt
                    let mut cnt = partial_proof_cnt.lock().unwrap();
                    let proof_cnt_this_round = circuit_copy_size * pack_size;
                    *cnt += proof_cnt_this_round;
                }
            })
        })
        .collect::<Vec<_>>();

    println!("We are now calculating average throughput, please wait for 5 seconds");
    for i in 0..args.repeats {
        thread::sleep(std::time::Duration::from_secs(5));
        let stop_time = std::time::Instant::now();
        let duration = stop_time.duration_since(start_time);
        let mut total_proof_cnt = 0;
        for cnt in &partial_proof_cnts {
            total_proof_cnt += *cnt.lock().unwrap();
        }
        let throughput =
            total_proof_cnt as f64 / duration.as_secs_f64() * (mpi_config.world_size() as f64);
        println!("{}-bench: throughput: {} hashes/s", i, throughput.round());
    }
}

fn print_info(args: &Args) {
    let prover = match args.circuit.as_str() {
        "keccak" => "GKR",
        "poseidon" => "GKR^2",
        _ => unreachable!(),
    };

    println!("===============================");
    println!(
        "benchmarking {} with {} over {}",
        args.circuit, prover, args.field
    );
    println!("field:          {}", args.field);
    println!("#threads:       {}", args.threads);
    println!("#bench repeats: {}", args.repeats);
    println!("hash circuit:   {}", args.circuit);
    println!("PCS:            {}", args.pcs);
    println!("===============================")
}
