// TODO: move this to `bench` repo and refactor with criterion
use std::{
    sync::{Arc, Mutex},
    thread,
};

use arith::{Field, VectorizedField, VectorizedFr, VectorizedM31, M31};
use clap::Parser;
use expander_rs::{Circuit, Config, Prover};
use halo2curves::bn256::Fr;

const FILENAME_MUL: &str = "data/ExtractedCircuitMul.txt";
const FILENAME_ADD: &str = "data/ExtractedCircuitAdd.txt";

// circuit for repeating Poseidon for 100 times
const POSEIDON_CIRCUIT: &str = "data/poseidon_100_circuit.txt";

/// ...
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Curve id: 0 for M31, 1 for Fr
    #[arg(short, long, default_value_t = 31)]
    field: usize,

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

    match args.field {
        31 => match args.scheme.as_str() {
            "keccak" => run_keccak_bench_m31(&args),
            "poseidon" => run_poseidon_bench_m31(&args),
            _ => unreachable!(),
        },
        254 => match args.scheme.as_str() {
            "keccak" => run_keccak_bench_fr(&args),

            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

fn run_poseidon_bench_m31(args: &Args) {
    let config = Config::m31_config();
    println!("benchmarking poseidon over {}", M31::NAME);
    println!(
        "Default parallel repetition config {}",
        config.get_num_repetitions()
    );

    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let start_time = std::time::Instant::now();

    // load circuit
    let circuit_template = Circuit::<VectorizedM31>::load_circuit(POSEIDON_CIRCUIT);
    // Circuit::<VectorizedM31>::load_extracted_gates(FILENAME_MUL, FILENAME_ADD);
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
                    const CIRCUIT_COPY_SIZE: usize = 100;
                    let proof_cnt_this_round = CIRCUIT_COPY_SIZE
                        * VectorizedM31::PACK_SIZE
                        * VectorizedM31::VECTORIZE_SIZE;
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
        println!("{}-bench: throughput: {} poseidon/s", i, throughput.round());
    }
}

fn run_keccak_bench_m31(args: &Args) {
    let config = Config::m31_config();
    println!("benchmarking keccak over {}", M31::NAME);
    println!(
        "Default parallel repetition config {}",
        config.get_num_repetitions()
    );

    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let start_time = std::time::Instant::now();

    // load circuit
    let circuit_template =
        Circuit::<VectorizedM31>::load_extracted_gates(FILENAME_MUL, FILENAME_ADD);
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
                    const CIRCUIT_COPY_SIZE: usize = 8;
                    let proof_cnt_this_round = CIRCUIT_COPY_SIZE
                        * VectorizedM31::PACK_SIZE
                        * VectorizedM31::VECTORIZE_SIZE;
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
        println!("{}-bench: throughput: {} keccaks/s", i, throughput.round());
    }
}

fn run_keccak_bench_fr(args: &Args) {
    let config = Config::bn254_config();
    println!("benchmarking keccak over {}", Fr::NAME);
    println!(
        "Default parallel repetition config {}",
        config.get_num_repetitions()
    );

    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let start_time = std::time::Instant::now();

    // load circuit
    let circuit_template =
        Circuit::<VectorizedFr>::load_extracted_gates(FILENAME_MUL, FILENAME_ADD);
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
                    const CIRCUIT_COPY_SIZE: usize = 8;
                    let proof_cnt_this_round =
                        CIRCUIT_COPY_SIZE * VectorizedFr::PACK_SIZE * VectorizedFr::VECTORIZE_SIZE;
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
        println!("{}-bench: throughput: {} keccaks/s", i, throughput.round());
    }
}

fn print_info(args: &Args) {
    println!("field:          {}", args.field);
    println!("#threads:       {}", args.threads);
    println!("#repeats:       {}", args.repeats);
}
