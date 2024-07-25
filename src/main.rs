// TODO: move this to `bench` repo and refactor with criterion
use std::{
    sync::{Arc, Mutex},
    thread,
};

use arith::{BinomialExtensionField, Bn254DummyExt3, SimdM31Ext3};
use arith::{FieldSerde, SimdField};
use clap::Parser;
use expander_rs::{Circuit, Config, Prover};

const KECCAK_CIRCUIT: &str = "data/circuit.txt";
// circuit for repeating Poseidon for 120 times
const POSEIDON_CIRCUIT: &str = "data/poseidon_120_circuit.txt";

const M31_PACKSIZE: usize = 8;
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
        "m31ext3" => run_benchmark::<SimdM31Ext3>(&args, Config::m31_ext3_config()),
        "fr" => run_benchmark::<Bn254DummyExt3>(&args, Config::bn254_config()),
        _ => unreachable!(),
    };
}

fn run_benchmark<F>(args: &Args, config: Config)
where
    F: BinomialExtensionField<3> + FieldSerde + SimdField + Send + 'static,
{
    println!("benchmarking keccak over {}", args.field);

    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let start_time = std::time::Instant::now();

    let pack_size = match args.field.as_str() {
        "m31ext3" => M31_PACKSIZE,
        "fr" => FR_PACKSIZE,
        _ => unreachable!(),
    };

    // load circuit
    let circuit_template = match args.scheme.as_str() {
        "keccak" => Circuit::<F>::load_circuit(KECCAK_CIRCUIT),
        "poseidon" => Circuit::<F>::load_circuit(POSEIDON_CIRCUIT),
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
            let mut local_config = config.clone();
            local_config.gkr_square = match args.scheme.as_str() {
                "keccak" => false,
                "poseidon" => true,
                _ => unreachable!(),
            };
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
    println!("field:          {}", args.field);
    println!("#threads:       {}", args.threads);
    println!("#bench repeats: {}", args.repeats);
    println!("scheme:         {}", args.scheme);
}
