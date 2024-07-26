// TODO: move this to `bench` repo and refactor with criterion
use std::{
    sync::{Arc, Mutex},
    thread,
};

use clap::Parser;
use expander_rs::{BN254Config, Circuit, Config, GKRConfig, GKRScheme, M31ExtConfig, Prover};

// circuit for repeating Keccak for 8 times
const KECCAK_CIRCUIT: &str = "data/circuit.txt";
// circuit for repeating Poseidon for 120 times
const POSEIDON_CIRCUIT: &str = "data/poseidon_120_circuit.txt";

const FR_PACKSIZE: usize = 1;

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
    #[arg(short, long, default_value_t = 4)]
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
            "keccak" => run_benchmark::<M31ExtConfig>(
                &args,
                Config::<M31ExtConfig>::new(GKRScheme::Vanilla),
            ),
            "poseidon" => run_benchmark::<M31ExtConfig>(
                &args,
                Config::<M31ExtConfig>::new(GKRScheme::GkrSquare),
            ),
            _ => unreachable!(),
        },
        "fr" => match args.scheme.as_str() {
            "keccak" => {
                run_benchmark::<BN254Config>(&args, Config::<BN254Config>::new(GKRScheme::Vanilla))
            }
            "poseidon" => run_benchmark::<BN254Config>(
                &args,
                Config::<BN254Config>::new(GKRScheme::GkrSquare),
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
    let start_time = std::time::Instant::now();
    #[cfg(target_arch = "x86_64")]
    let m31_packsize: usize = 16;
    #[cfg(target_arch = "aarch64")]
    let m31_packsize: usize = 8;
    let pack_size = match args.field.as_str() {
        "m31ext3" => m31_packsize,
        "fr" => FR_PACKSIZE,
        _ => unreachable!(),
    };

    // load circuit
    let circuit_template = match args.scheme.as_str() {
        "keccak" => Circuit::<C>::load_circuit(KECCAK_CIRCUIT),
        "poseidon" => Circuit::<C>::load_circuit(POSEIDON_CIRCUIT),
        _ => unreachable!(),
    };

    let circuit_copy_size: usize = match args.scheme.as_str() {
        "keccak" => 8,
        "poseidon" => 120,
        _ => unreachable!(),
    };

    let circuits = (0..args.threads)
        .map(|_| {
            let mut c = circuit_template.clone();
            c.set_random_bool_input_for_test();
            c.evaluate();
            c
        })
        .collect::<Vec<_>>();

    println!("Circuit loaded!");
    let _ = circuits
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let partial_proof_cnt = partial_proof_cnts[i].clone();
            let local_config = config.clone();
            thread::spawn(move || {
                loop {
                    // bench func
                    let mut prover = Prover::new(&local_config);
                    prover.prepare_mem(&c);
                    prover.prove(&c);
                    // update cnt
                    let mut cnt = partial_proof_cnt.lock().unwrap();
                    let proof_cnt_this_round = circuit_copy_size * pack_size;
                    *cnt += proof_cnt_this_round;
                }
            })
        })
        .collect::<Vec<_>>();

    println!("We are now calculating average throughput, please wait for 1 minutes");
    for i in 0..args.repeats {
        thread::sleep(std::time::Duration::from_secs(60));
        let stop_time = std::time::Instant::now();
        let duration = stop_time.duration_since(start_time);
        let mut total_proof_cnt = 0;
        for cnt in &partial_proof_cnts {
            total_proof_cnt += *cnt.lock().unwrap();
        }
        let throughput = total_proof_cnt as f64 / duration.as_secs_f64();
        println!("{}-bench: throughput: {} hashes/s", i, throughput.round());
    }
}

fn print_info(args: &Args) {
    println!("===============================");
    println!(
        "benchmarking {} with GKR^2 over {}",
        args.scheme, args.field
    );
    println!("field:          {}", args.field);
    println!("#threads:       {}", args.threads);
    println!("#bench repeats: {}", args.repeats);
    println!("hash scheme:    {}", args.scheme);
    println!("===============================")
}
