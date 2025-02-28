use circuit::Circuit;
use clap::Parser;
use config::{Config, GKRConfig, GKRScheme};
use mpi_config::MPIConfig;

use gkr_field_config::GKRFieldConfig;
use poly_commit::expander_pcs_init_testing_only;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use gkr::{
    utils::{
        KECCAK_BN254_CIRCUIT, KECCAK_BN254_WITNESS, KECCAK_GF2_CIRCUIT, KECCAK_GF2_WITNESS,
        KECCAK_M31_CIRCUIT, KECCAK_M31_WITNESS, POSEIDON_M31_CIRCUIT, POSEIDON_M31_WITNESS,
    },
    BN254ConfigSha2Hyrax, GF2ExtConfigSha2Orion, M31ExtConfigSha2Orion, Prover,
};

#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;

/// ...
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Field Identifier: fr, m31, m31ext3
    #[arg(short, long,default_value_t = String::from("m31ext3"))]
    field: String,

    // scheme: keccak, poseidon
    #[arg(short, long, default_value_t = String::from("keccak"))]
    scheme: String,

    /// number of repeat
    #[arg(short, long, default_value_t = 1)]
    repeats: usize,
}

fn main() {
    let args = Args::parse();
    print_info(&args);

    let mpi_config = MPIConfig::new();

    match args.field.as_str() {
        "m31ext3" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<M31ExtConfigSha2Orion>(
                &args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            ),
            "poseidon" => run_benchmark::<M31ExtConfigSha2Orion>(
                &args,
                Config::new(GKRScheme::GkrSquare, mpi_config.clone()),
            ),
            _ => unreachable!(),
        },
        "fr" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<BN254ConfigSha2Hyrax>(
                &args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            ),
            "poseidon" => run_benchmark::<BN254ConfigSha2Hyrax>(
                &args,
                Config::new(GKRScheme::GkrSquare, mpi_config.clone()),
            ),
            _ => unreachable!(),
        },
        "gf2ext128" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<GF2ExtConfigSha2Orion>(
                &args,
                Config::new(GKRScheme::Vanilla, mpi_config.clone()),
            ),
            "poseidon" => run_benchmark::<GF2ExtConfigSha2Orion>(
                &args,
                Config::new(GKRScheme::GkrSquare, mpi_config.clone()),
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    MPIConfig::finalize();
}

const PCS_TESTING_SEED_U64: u64 = 114514;

fn run_benchmark<Cfg: GKRConfig>(args: &Args, config: Config<Cfg>) {
    let pack_size = Cfg::FieldConfig::get_field_pack_size();

    // load circuit
    let mut circuit = match args.scheme.as_str() {
        "keccak" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::GF2 => Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(KECCAK_GF2_CIRCUIT),
            FieldType::M31 => Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(KECCAK_M31_CIRCUIT),
            FieldType::BN254 => {
                Circuit::<Cfg::FieldConfig>::load_circuit::<Cfg>(KECCAK_BN254_CIRCUIT)
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

    let witness_path = match args.scheme.as_str() {
        "keccak" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::GF2 => KECCAK_GF2_WITNESS,
            FieldType::M31 => KECCAK_M31_WITNESS,
            FieldType::BN254 => KECCAK_BN254_WITNESS,
        },
        "poseidon" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::M31 => POSEIDON_M31_WITNESS,
            _ => unreachable!("not supported"),
        },
        _ => unreachable!(),
    };

    circuit.load_witness_allow_padding_testing_only(witness_path, &config.mpi_config);

    let circuit_copy_size: usize = match (Cfg::FieldConfig::FIELD_TYPE, args.scheme.as_str()) {
        (FieldType::GF2, "keccak") => 1,
        (FieldType::M31, "keccak") => 2,
        (FieldType::BN254, "keccak") => 2,
        (FieldType::M31, "poseidon") => 120,
        _ => unreachable!(),
    };

    let mut prover = Prover::new(&config);
    prover.prepare_mem(&circuit);

    let mut rng = ChaCha12Rng::seed_from_u64(PCS_TESTING_SEED_U64);
    let (pcs_params, pcs_proving_key, _pcs_verification_key, mut pcs_scratch) =
        expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::Transcript, Cfg::PCS>(
            circuit.log_input_size(),
            &config.mpi_config,
            &mut rng,
        );

    const N_PROOF: usize = 1000;

    println!("We are now calculating average throughput, please wait until {N_PROOF} proofs are computed");
    for i in 0..args.repeats {
        config.mpi_config.barrier(); // wait until everyone is here
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
        let throughput = (N_PROOF * circuit_copy_size * pack_size * config.mpi_config.world_size())
            as f64
            / duration.as_secs_f64();
        println!("{}-bench: throughput: {} hashes/s", i, throughput.round());
    }
}

fn print_info(args: &Args) {
    let prover = match args.scheme.as_str() {
        "keccak" => "GKR",
        "poseidon" => "GKR^2",
        _ => unreachable!(),
    };

    println!("===============================");
    println!(
        "benchmarking {} with {} over {}",
        args.scheme, prover, args.field
    );
    println!("field:          {}", args.field);
    println!("#bench repeats: {}", args.repeats);
    println!("hash scheme:    {}", args.scheme);
    println!("===============================")
}
