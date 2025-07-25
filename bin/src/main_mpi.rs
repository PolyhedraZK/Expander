use std::str::FromStr;

use circuit::Circuit;
use clap::Parser;
use gkr::{
    BN254ConfigMIMC5KZG, BN254ConfigSha2Hyrax, BN254ConfigSha2Raw, GF2ExtConfigSha2Orion,
    GF2ExtConfigSha2Raw, Goldilocksx8ConfigSha2Orion, Goldilocksx8ConfigSha2Raw,
    M31x16ConfigSha2OrionSquare, M31x16ConfigSha2OrionVanilla, M31x16ConfigSha2RawSquare,
    M31x16ConfigSha2RawVanilla, Prover,
    utils::{
        KECCAK_BABYBEAR_CIRCUIT, KECCAK_BABYBEAR_WITNESS, KECCAK_BN254_CIRCUIT,
        KECCAK_BN254_WITNESS, KECCAK_GF2_CIRCUIT, KECCAK_GF2_WITNESS, KECCAK_GOLDILOCKS_CIRCUIT,
        KECCAK_GOLDILOCKS_WITNESS, KECCAK_M31_CIRCUIT, KECCAK_M31_WITNESS, POSEIDON_M31_CIRCUIT,
        POSEIDON_M31_WITNESS,
    },
};
use gkr_engine::{
    FieldEngine, FieldType, GKREngine, MPIConfig, MPIEngine, MPISharedMemory,
    PolynomialCommitmentType, root_println,
};
use poly_commit::expander_pcs_init_testing_only;
use serdes::ExpSerde;

/// ...
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Field Identifier: fr, m31, m31ext3
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
}

fn main() {
    let args = Args::parse();

    let universe = MPIConfig::init().unwrap();
    let world = universe.world();
    let mpi_config = MPIConfig::prover_new(Some(&universe), Some(&world));

    print_info(&args, &mpi_config);

    let pcs_type = PolynomialCommitmentType::from_str(&args.pcs).unwrap();

    match args.field.as_str() {
        "m31" => match pcs_type {
            PolynomialCommitmentType::Raw => match args.circuit.as_str() {
                "keccak" => run_benchmark::<M31x16ConfigSha2RawVanilla>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            _ => unreachable!("Unsupported PCS type for M31"),
        },
        "m31ext3" => match pcs_type {
            PolynomialCommitmentType::Raw => match args.circuit.as_str() {
                "keccak" => run_benchmark::<M31x16ConfigSha2RawVanilla>(&args, mpi_config.clone()),
                "poseidon" => run_benchmark::<M31x16ConfigSha2RawSquare>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            PolynomialCommitmentType::Orion => match args.circuit.as_str() {
                "keccak" => {
                    run_benchmark::<M31x16ConfigSha2OrionVanilla>(&args, mpi_config.clone())
                }
                "poseidon" => {
                    run_benchmark::<M31x16ConfigSha2OrionSquare>(&args, mpi_config.clone())
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
                "keccak" => run_benchmark::<Goldilocksx8ConfigSha2Raw>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            PolynomialCommitmentType::Orion => match args.circuit.as_str() {
                "keccak" => run_benchmark::<Goldilocksx8ConfigSha2Orion>(&args, mpi_config.clone()),
                _ => unreachable!(),
            },
            _ => unreachable!("Unsupported PCS type for Goldilocks"),
        },
        _ => unreachable!(),
    };
}

fn run_benchmark<Cfg: GKREngine>(args: &Args, mpi_config: MPIConfig)
where
    Cfg::FieldConfig: FieldEngine,
{
    let pack_size = <Cfg::FieldConfig as FieldEngine>::get_field_pack_size();

    // load circuit
    let (mut circuit, mut window) = match args.circuit.as_str() {
        "keccak" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::GF2Ext128 => Circuit::<Cfg::FieldConfig>::prover_load_circuit::<Cfg>(
                KECCAK_GF2_CIRCUIT,
                &mpi_config,
            ),
            FieldType::M31x1 => {
                unimplemented!("x1 mod only used for testing. use main instead of main_mpi")
            }
            FieldType::M31x16 => Circuit::<Cfg::FieldConfig>::prover_load_circuit::<Cfg>(
                KECCAK_M31_CIRCUIT,
                &mpi_config,
            ),
            FieldType::BN254 => Circuit::<Cfg::FieldConfig>::prover_load_circuit::<Cfg>(
                KECCAK_BN254_CIRCUIT,
                &mpi_config,
            ),
            FieldType::Goldilocksx1 => {
                unimplemented!("x1 mod only used for testing. use main instead of main_mpi")
            }
            FieldType::Goldilocksx8 => Circuit::<Cfg::FieldConfig>::prover_load_circuit::<Cfg>(
                KECCAK_GOLDILOCKS_CIRCUIT,
                &mpi_config,
            ),
            FieldType::BabyBearx16 => Circuit::<Cfg::FieldConfig>::prover_load_circuit::<Cfg>(
                KECCAK_BABYBEAR_CIRCUIT,
                &mpi_config,
            ),
        },
        "poseidon" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::M31x16 => Circuit::<Cfg::FieldConfig>::prover_load_circuit::<Cfg>(
                POSEIDON_M31_CIRCUIT,
                &mpi_config,
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    let witness_path = match args.circuit.as_str() {
        "keccak" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::GF2Ext128 => KECCAK_GF2_WITNESS,
            FieldType::M31x1 => {
                unimplemented!("x1 mod only used for testing. use main instead of main_mpi")
            }
            FieldType::M31x16 => KECCAK_M31_WITNESS,
            FieldType::BN254 => KECCAK_BN254_WITNESS,
            FieldType::Goldilocksx1 => {
                unimplemented!("x1 mod only used for testing. use main instead of main_mpi")
            }
            FieldType::Goldilocksx8 => KECCAK_GOLDILOCKS_WITNESS,
            FieldType::BabyBearx16 => KECCAK_BABYBEAR_WITNESS,
        },
        "poseidon" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::M31x16 => POSEIDON_M31_WITNESS,
            _ => unreachable!("not supported"),
        },
        _ => unreachable!(),
    };

    circuit.load_witness_allow_padding_testing_only(witness_path, &mpi_config);

    let circuit_copy_size: usize = match (Cfg::FieldConfig::FIELD_TYPE, args.circuit.as_str()) {
        (FieldType::GF2Ext128, "keccak") => 1,
        (FieldType::M31x16, "keccak") => 2,
        (FieldType::BN254, "keccak") => 2,
        (FieldType::M31x16, "poseidon") => 120,
        (FieldType::Goldilocksx8, "keccak") => 2,
        _ => unreachable!(),
    };

    let mut prover = Prover::<Cfg>::new(mpi_config.clone());
    prover.prepare_mem(&circuit);

    let (pcs_params, pcs_proving_key, _pcs_verification_key, mut pcs_scratch) =
        expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::PCSConfig>(
            circuit.log_input_size(),
            &mpi_config,
        );

    // calculate the proof size
    {
        let (claim, proof) = prover.prove(
            &mut circuit,
            &pcs_params,
            &pcs_proving_key,
            &mut pcs_scratch,
        );
        let mut buf = Vec::new();
        claim.serialize_into(&mut buf).unwrap();
        proof.serialize_into(&mut buf).unwrap();
        root_println!(mpi_config, "Proof size: {}", buf.len());

        // Write the serialized proof to a file
        std::fs::write("proof_mpi.txt", &buf).expect("Failed to write proof to file");
        println!("Proof written to proof_mpi.txt");
    }

    const N_PROOF: usize = 10;

    root_println!(
        mpi_config,
        "We are now calculating average throughput, please wait until {N_PROOF} proofs are computed"
    );
    for i in 0..args.repeats {
        mpi_config.barrier(); // wait until everyone is here
        let start_time = std::time::Instant::now();
        for _j in 0..N_PROOF {
            prover.prove(
                &mut circuit,
                &pcs_params,
                &pcs_proving_key,
                &mut pcs_scratch,
            );
        }
        let stop_time = std::time::Instant::now();
        let duration = stop_time.duration_since(start_time);
        let throughput = (N_PROOF * circuit_copy_size * pack_size * mpi_config.world_size()) as f64
            / duration.as_secs_f64();
        root_println!(
            mpi_config,
            "{}-bench: throughput: {} hashes/s",
            i,
            throughput.round()
        );
    }
    circuit.discard_control_of_shared_mem();
    mpi_config.free_shared_mem(&mut window);
}

fn print_info(args: &Args, mpi_config: &MPIConfig) {
    if !mpi_config.is_root() {
        return;
    }

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
    println!("#bench repeats: {}", args.repeats);
    println!("hash circuit:   {}", args.circuit);
    println!("PCS:            {}", args.pcs);
    println!("===============================")
}
