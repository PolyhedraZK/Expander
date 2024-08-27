use std::fs;
use std::path::Path;
use std::process::Command;

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

fn download_if_not_exists(url: &str, file: &str) {
    if !Path::new(file).exists() {
        println!("Downloading {}", file);
        let download = Command::new("bash")
            .arg("-c")
            .arg(format!("wget {url} -O {file}"))
            .output()
            .expect("Failed to download circuit");

        assert!(download.status.success(), "Circuit download failure");
    } else {
        println!("{} already exists, skipping download", file);
    }
}

pub fn dev_env_data_setup() {
    fs::create_dir_all(DATA_PREFIX).unwrap();
    download_if_not_exists(KECCAK_URL, KECCAK_CIRCUIT);
    download_if_not_exists(KECCAK_WITNESS_URL, KECCAK_WITNESS);
    download_if_not_exists(POSEIDON_URL, POSEIDON_CIRCUIT);
}

#[allow(dead_code)]
fn main() {
    dev_env_data_setup()
}
