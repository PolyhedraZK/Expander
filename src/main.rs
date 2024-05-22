// TODO: move this to `bench` repo and refactor with criterion
use std::{
    env,
    sync::{Arc, Mutex},
    thread,
};

use expander_rs::{
    m31::{M31_PACK_SIZE, M31_VECTORIZE_SIZE},
    Circuit, Config, Prover,
};

const FILENAME_MUL: &str = "data/ExtractedCircuitMul.txt";
const FILENAME_ADD: &str = "data/ExtractedCircuitAdd.txt";

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let num_thread = if args.len() <= 1 {
        println!("Use `cargo run -- <number_of_threads>`. Default to 4.");
        4
    } else {
        let v = args[1].parse::<usize>().unwrap();
        assert_ne!(v, 0, "Argumemt #1 number_of_threads is incorrect.");
        v
    };
    println!("Benchmarking with {} threads", num_thread);

    let local_config = Config::new();
    println!(
        "Default parallel repetition config {}",
        local_config.get_num_repetitions()
    );

    let partial_proof_cnts = (0..num_thread)
        .map(|_| Arc::new(Mutex::new(0)))
        .collect::<Vec<_>>();
    let start_time = std::time::Instant::now();

    // load circuit
    let circuit_template = Circuit::load_extracted_gates(FILENAME_MUL, FILENAME_ADD);
    let circuits = (0..num_thread)
        .map(|_| {
            let mut c = circuit_template.clone();
            c.set_random_bool_input();
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
            let local_config = local_config.clone();
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
                        CIRCUIT_COPY_SIZE * M31_PACK_SIZE * M31_VECTORIZE_SIZE;
                    *cnt += proof_cnt_this_round;
                }
            })
        })
        .collect::<Vec<_>>();

    println!("We are now calculating average throughput, please wait for 1 minutes");
    loop {
        thread::sleep(std::time::Duration::from_secs(60));
        let stop_time = std::time::Instant::now();
        let duration = stop_time.duration_since(start_time);
        let mut total_proof_cnt = 0;
        for cnt in &partial_proof_cnts {
            total_proof_cnt += *cnt.lock().unwrap();
        }
        let throughput = total_proof_cnt as f64 / duration.as_secs_f64();
        println!("Throughput: {} keccaks/s", throughput.round());
    }
}
