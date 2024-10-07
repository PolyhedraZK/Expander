use std::{
    sync::{Arc, Mutex},
    thread,
};

use clap::Parser;
use expander_rs::utils::{KECCAK_GF2_CIRCUIT, KECCAK_M31_CIRCUIT, POSEIDON_CIRCUIT, BENCH_PERIOD, PAPER_CIRCUIT};
use expander_rs::{
    BN254ConfigSha2, Circuit, Config, FieldType, GF2ExtConfigSha2, GKRConfig, GKRScheme,
    M31ExtConfigSha2, Prover,
};

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

    match args.field.as_str() {
        "m31ext3" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<M31ExtConfigSha2>(
                &args,
                Config::<M31ExtConfigSha2>::new(GKRScheme::Vanilla),
            ),
            "poseidon" => run_benchmark::<M31ExtConfigSha2>(
                &args,
                Config::<M31ExtConfigSha2>::new(GKRScheme::GkrSquare),
            ),
            "paper" => run_benchmark::<M31ExtConfigSha2>(
                &args,
                Config::<M31ExtConfigSha2>::new(GKRScheme::Vanilla),
            ),
            _ => unreachable!(),
        },
        "fr" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<BN254ConfigSha2>(
                &args,
                Config::<BN254ConfigSha2>::new(GKRScheme::Vanilla),
            ),
            "poseidon" => run_benchmark::<BN254ConfigSha2>(
                &args,
                Config::<BN254ConfigSha2>::new(GKRScheme::GkrSquare),
            ),
            _ => unreachable!(),
        },
        "gf2ext128" => match args.scheme.as_str() {
            "keccak" => run_benchmark::<GF2ExtConfigSha2>(
                &args,
                Config::<GF2ExtConfigSha2>::new(GKRScheme::Vanilla),
            ),
            "poseidon" => run_benchmark::<GF2ExtConfigSha2>(
                &args,
                Config::<GF2ExtConfigSha2>::new(GKRScheme::GkrSquare),
            ),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
}

fn run_benchmark<C: GKRConfig>(args: &Args, config: Config<C>) {
    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let pack_size = C::get_field_pack_size();
    println!("pack_size: {}", pack_size);

    // load circuit
    let mut circuit_template = match args.scheme.as_str() {
        "keccak" => match C::FIELD_TYPE {
            FieldType::GF2 => Circuit::<C>::load_circuit(KECCAK_GF2_CIRCUIT),
            FieldType::M31 => Circuit::<C>::load_circuit(KECCAK_M31_CIRCUIT),
            FieldType::BN254 => Circuit::<C>::load_circuit(KECCAK_M31_CIRCUIT),
        },
        "poseidon" => Circuit::<C>::load_circuit(POSEIDON_CIRCUIT),
        "paper" => Circuit::<C>::load_circuit(PAPER_CIRCUIT),
        _ => unreachable!(),
    };

    println!("Circuit loaded!");
    let circuit_copy_size: usize = match (C::FIELD_TYPE, args.scheme.as_str()) {
        (FieldType::GF2, "keccak") => 1,
        (FieldType::M31, "keccak") => 2,
        (FieldType::BN254, "keccak") => 2,
        (FieldType::M31, "poseidon") => 120,
        (FieldType::BN254, "poseidon") => 120,
        (FieldType::M31, "paper") => 1,
        _ => unreachable!(),
    };

    circuit_template.set_random_input_for_test();
    circuit_template.evaluate();
    println!("circuits evaluated!");
    let config_arc = Arc::new(config);
    let circuit_ptr = { 
        let address = &circuit_template as *const Circuit<C>;
        let address_usize = address as usize;
        address_usize
    };
    let wait = (0..args.threads)
        .map(|i| {
            let local_config = Arc::clone(&config_arc);
            thread::spawn(move || {
                let c = unsafe { &*(circuit_ptr as *const Circuit<C>) };
                // bench func
                let mut prover = Prover::new(&local_config);
                prover.prepare_mem(&c);
                println!("thread {} prepared!", i);
                let start_time = std::time::Instant::now();
                prover.prove(&c);
                let elapsed = start_time.elapsed();
                println!(
                    "elapsed time: {}.{:03} s",
                    elapsed.as_secs(),
                    elapsed.subsec_millis()
                );
            })
        })
        .collect::<Vec<_>>();
    for w in wait {
        w.join().unwrap();
    }
}

fn print_info(args: &Args) {
    let prover = match args.scheme.as_str() {
        "keccak" => "GKR",
        "poseidon" => "GKR^2",
        "paper" => "GKR",
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
