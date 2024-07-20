// TODO: move this to `bench` repo and refactor with criterion
use std::{
    sync::{Arc, Mutex},
    thread,
};

use arith::{Field, FieldSerde, VectorizedM31};
use clap::Parser;
use expander_rs::{Circuit, Config, Prover};
use halo2curves::bn256::Fr;

// #[cfg(target_arch = "x86_64")]
// use arith::VectorizedM31Ext3;

const FILENAME_MUL: &str = "data/ExtractedCircuitMul.txt";
const FILENAME_ADD: &str = "data/ExtractedCircuitAdd.txt";

const CIRCUIT_COPY_SIZE: usize = 8;
const M31_PACKSIZE: usize = 16;
const FR_PACKSIZE: usize = 1;

/// ...
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Curve id
    #[arg(short, long,default_value_t = String::from("fr"))]
    field: String,

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
        "m31" => run_benchmark::<VectorizedM31>(&args, Config::m31_config()),
        // #[cfg(target_arch = "x86_64")]
        // "m31ext3" => run_benchmark::<VectorizedM31Ext3>(&args, Config::m31_ext3_config()),
        "fr" => run_benchmark::<Fr>(&args, Config::bn254_config()),
        _ => unreachable!(),
    };
}

fn run_benchmark<VecF>(args: &Args, config: Config)
where
    VecF: Field + FieldSerde + Send + 'static,
{
    println!("benchmarking keccak over {}", args.field);
    println!(
        "Default parallel repetition config {}",
        config.get_num_repetitions()
    );

    let partial_proof_cnts = (0..args.threads)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let start_time = std::time::Instant::now();

    let pack_size = match args.field.as_str() {
        "m31" => M31_PACKSIZE,
        "fr" => FR_PACKSIZE,
        _ => unreachable!(),
    };

    // load circuit
    let circuit_template = Circuit::<VecF>::load_extracted_gates(FILENAME_MUL, FILENAME_ADD);
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

                    let proof_cnt_this_round = CIRCUIT_COPY_SIZE * pack_size;
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
    println!("#bench repeats: {}", args.repeats);
}
