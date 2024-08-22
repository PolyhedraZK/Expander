use std::{
    fs::{self, OpenOptions},
    io::Write,
};

const DATA_PREFIX: &str = "data/";

// circuit for repeating Keccak for 2 times
pub const KECCAK_CIRCUIT: &str = "data/circuit.txt";
// URL for Keccak circuit repeated for 2 times
pub const KECCAK_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/keccak_2_circuit.txt";

pub const KECCAK_WITNESS: &str = "data/witness.txt";
pub const KECCAK_WITNESS_URL: &str = "https://storage.googleapis.com/keccak8/witness.txt";

// circuit for repeating Poseidon for 120 times
pub const POSEIDON_CIRCUIT: &str = "data/poseidon_120_circuit.txt";
// URL for Poseidon circuit repeated for 120 times
pub const POSEIDON_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/poseidon_120_circuit.txt";

fn download_and_store(url: &str, file: &str) {
    let resp = reqwest::blocking::get(url).expect("reqwest failed");
    let body = resp.bytes().expect("body invalid");

    let mut io_out = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(file)
        .expect("failed to open or create file");

    io_out.write_all(&body).expect("failed to copy content");
}

pub fn dev_env_data_setup() {
    fs::create_dir_all(DATA_PREFIX).unwrap();
    download_and_store(KECCAK_URL, KECCAK_CIRCUIT);
    download_and_store(KECCAK_WITNESS_URL, KECCAK_WITNESS);
    download_and_store(POSEIDON_URL, POSEIDON_CIRCUIT);
}

#[allow(dead_code)]
fn main() {
    dev_env_data_setup()
}
