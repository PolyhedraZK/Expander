use std::{
    sync::{Arc, Mutex},
    thread,
};

use circuit::Circuit;
use clap::Parser;
use config::{Config, GKRConfig, GKRScheme};
use config_macros::declare_gkr_config;
use gkr_field_config::{BN254Config, GF2ExtConfig, GKRFieldConfig, M31ExtConfig};
use communicator::{MPICommunicator, ExpanderComm};

use poly_commit::{expander_pcs_init_testing_only, raw::RawExpanderGKR};
use transcript::{BytesHashTranscript, SHA256hasher};

use gkr::{
    utils::{
        KECCAK_BN254_CIRCUIT, KECCAK_BN254_WITNESS, KECCAK_GF2_CIRCUIT, KECCAK_GF2_WITNESS,
        KECCAK_M31_CIRCUIT, KECCAK_M31_WITNESS, POSEIDON_M31_CIRCUIT, POSEIDON_M31_WITNESS,
    },
    Prover,
};

#[allow(unused_imports)]
// The FiatShamirHashType and PolynomialCommitmentType import is used in the macro expansion
use config::{FiatShamirHashType, PolynomialCommitmentType};
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

    /// number of thread
    #[arg(short, long, default_value_t = 1)]
    threads: u64,
}

fn main() {
    let args = Args::parse();
    print_info(&args);

    let mpi_comm = MPICommunicator::new(1);

    declare_gkr_config!(
        M31ExtConfigSha2,
        FieldType::M31,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        BN254ConfigSha2,
        FieldType::BN254,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );
    declare_gkr_config!(
        GF2ExtConfigSha2,
        FieldType::GF2,
        FiatShamirHashType::SHA256,
        PolynomialCommitmentType::Raw
    );

    match args.field.as_str() {
        "m31ext3" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<M31ExtConfigSha2>(
                &args,
                Config::<M31ExtConfigSha2>::new(GKRScheme::Vanilla, &&mpi_comm.clone()),
            ),
            "poseidon" => run_benchmark::<M31ExtConfigSha2>(
                &args,
                Config::<M31ExtConfigSha2>::new(GKRScheme::GkrSquare, &&mpi_comm.clone()),
            ),
            _ => unreachable!(),
        },
        "fr" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<BN254ConfigSha2>(
                &args,
                Config::<BN254ConfigSha2>::new(GKRScheme::Vanilla, &&mpi_comm.clone()),
            ),
            "poseidon" => run_benchmark::<BN254ConfigSha2>(
                &args,
                Config::<BN254ConfigSha2>::new(GKRScheme::GkrSquare, &&mpi_comm.clone()),
            ),
            _ => unreachable!(),
        },
        "gf2ext128" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<GF2ExtConfigSha2>(
                &args,
                Config::<GF2ExtConfigSha2>::new(GKRScheme::Vanilla, &&mpi_comm.clone()),
            ),
            "poseidon" => run_benchmark::<GF2ExtConfigSha2>(
                &args,
                Config::<GF2ExtConfigSha2>::new(GKRScheme::GkrSquare, &&mpi_comm.clone()),
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    MPICommunicator::finalize();
}

fn run_benchmark<Cfg: GKRConfig>(args: &Args, config: Config<Cfg>) {
    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let pack_size = Cfg::FieldConfig::get_field_pack_size();

    // load circuit
    let mut circuit_template = match args.scheme.as_str() {
        "keccak" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::GF2 => Circuit::<Cfg::FieldConfig>::load_circuit(KECCAK_GF2_CIRCUIT),
            FieldType::M31 => Circuit::<Cfg::FieldConfig>::load_circuit(KECCAK_M31_CIRCUIT),
            FieldType::BN254 => Circuit::<Cfg::FieldConfig>::load_circuit(KECCAK_BN254_CIRCUIT),
        },
        "poseidon" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::M31 => Circuit::<Cfg::FieldConfig>::load_circuit(POSEIDON_M31_CIRCUIT),
            _ => unreachable!("not supported"),
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

    match args.scheme.as_str() {
        "keccak" => circuit_template.load_witness_file(witness_path),
        "poseidon" => match Cfg::FieldConfig::FIELD_TYPE {
            FieldType::M31 => circuit_template.load_non_simd_witness_file(witness_path),
            _ => unreachable!("not supported"),
        },

        _ => unreachable!(),
    };

    let circuit_copy_size: usize = match (Cfg::FieldConfig::FIELD_TYPE, args.scheme.as_str()) {
        (FieldType::GF2, "keccak") => 1,
        (FieldType::M31, "keccak") => 2,
        (FieldType::BN254, "keccak") => 2,
        (FieldType::M31, "poseidon") => 120,
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

    let (pcs_params, pcs_proving_key, _pcs_verification_key, pcs_scratch) =
        expander_pcs_init_testing_only::<Cfg::FieldConfig, Cfg::Transcript, Cfg::PCS>(
            circuit_template.log_input_size(),
            &config.comm,
        );

    let start_time = std::time::Instant::now();
    let _ = circuits
        .into_iter()
        .enumerate()
        .map(|(i, mut c)| {
            let partial_proof_cnt = partial_proof_cnts[i].clone();
            let local_config = config.clone();
            let pcs_params = pcs_params.clone();
            let pcs_proving_key = pcs_proving_key.clone();
            let mut pcs_scratch = pcs_scratch.clone();
            thread::spawn(move || {
                // bench func
                let mut prover = Prover::new(&local_config);
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
        let throughput = total_proof_cnt as f64 / duration.as_secs_f64()
            * (config.comm.world_size() as f64);
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
    println!("#threads:       {}", args.threads);
    println!("#bench repeats: {}", args.repeats);
    println!("hash scheme:    {}", args.scheme);
    println!("===============================")
}
