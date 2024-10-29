use std::fs;
use std::process::Command;

const DATA_PREFIX: &str = "data/";

// circuit for repeating Keccak for 2 times
// pub const KECCAK_M31_CIRCUIT: &str = "data/circuit_m31.txt";
// pub const KECCAK_M31_CIRCUIT: &str = "../../gnark-bls12_381/gkr/circuit_pairing.txt";
// pub const KECCAK_M31_CIRCUIT: &str = "../../gnark-bls12_381/gkr/circuit_ate2.txt";
pub const KECCAK_M31_CIRCUIT: &str = "../../EthFullConsensus/consensus/shuffle/gkr/circuit_shufflewithhashmap32range.txt";
// pub const KECCAK_M31_CIRCUIT: &str = "../../EthFullConsensus/consensus/hashmap/gkr/circuit.txt";
// pub const KECCAK_M31_CIRCUIT: &str = "../../ExpanderCompilerCollection/ecgo/examples/poseidon_m31/poseidon_120_circuit_m31.txt";
// pub const KECCAK_M31_CIRCUIT: &str = "../../EthFullConsensus/consensus/shuffle/gkr/circuit_prepare64.txt";
// pub const KECCAK_M31_CIRCUIT: &str = "/home/hc/gnark_m31/std/algebra/emulated/sw_bls12381_m31/circuit_aggpubkey_pairing.txt";
// pub const KECCAK_M31_CIRCUIT: &str = "/home/hc/gnark-bls12_381/gkr/circuit_aggpubkey_And_pairing.txt";
pub const KECCAK_GF2_CIRCUIT: &str = "data/circuit_gf2.txt";
pub const KECCAK_BN254_CIRCUIT: &str = "data/circuit_bn254.txt";

// pub const KECCAK_M31_WITNESS: &str = "data/witness_m31.txt";
// pub const KECCAK_M31_WITNESS: &str = "../../gnark-bls12_381/gkr/witness_pairing.txt";
// pub const KECCAK_M31_WITNESS: &str = "../../gnark-bls12_381/gkr/witness_ate2.txt";
pub const KECCAK_M31_WITNESS: &str = "../../EthFullConsensus/consensus/shuffle/gkr/witness_shufflewithhashmap32range.txt";
// pub const KECCAK_M31_WITNESS: &str = "../../EthFullConsensus/consensus/hashmap/gkr/witness.txt";
// pub const KECCAK_M31_WITNESS: &str =  "../../ExpanderCompilerCollection/ecgo/examples/poseidon_m31/poseidon_120_witness_m31.txt";
// pub const KECCAK_M31_WITNESS: &str = "../../EthFullConsensus/consensus/shuffle/gkr/witness_prepare64.txt";
// pub const KECCAK_M31_WITNESS: &str = "/home/hc/gnark_m31/std/algebra/emulated/sw_bls12381_m31/witness_aggpubkey_pairing.txt";
// pub const KECCAK_M31_WITNESS: &str = "/home/hc/gnark-bls12_381/gkr/witness_aggpubkey_And_pairing.txt";
pub const KECCAK_GF2_WITNESS: &str = "data/witness_gf2.txt";
pub const KECCAK_BN254_WITNESS: &str = "data/witness_bn254.txt";

// URL for Keccak circuit repeated for 2 times
pub const KECCAK_CIRCUIT_M31_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/keccak-ci/serialization-v6/circuit_m31.txt";
pub const KECCAK_CIRCUIT_GF2_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/keccak-ci/serialization-v6/circuit_gf2.txt";
pub const KECCAK_CIRCUIT_BN254_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/keccak-ci/serialization-v6/circuit_bn254.txt";

pub const KECCAK_WITNESS_M31_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/keccak-ci/serialization-v6/witness_m31.txt";
pub const KECCAK_WITNESS_GF2_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/keccak-ci/serialization-v6/witness_gf2.txt";
pub const KECCAK_WITNESS_BN254_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/keccak-ci/serialization-v6/witness_bn254.txt";

// circuit for repeating Poseidon for 120 times
pub const POSEIDON_M31_CIRCUIT: &str = "data/poseidon_120_circuit_m31.txt";
// circuit for repeating Poseidon for 120 times
pub const POSEIDON_M31_WITNESS: &str = "data/poseidon_120_witness_m31.txt";
// // circuit for repeating Poseidon for 120 times
// pub const POSEIDON_BN254_CIRCUIT: &str = "data/poseidon_120_circuit_bn254.txt";
// // circuit for repeating Poseidon for 120 times
// pub const POSEIDON_BN254_WITNESS: &str = "data/poseidon_120_witness_bn254.txt";

// URL for Poseidon circuit repeated for 120 times
pub const POSEIDON_CIRCUIT_M31_URL: &str =
    "https://storage.googleapis.com/expander-compiled-circuits/poseidon-ci/poseidon_120_circuit_m31.txt";
// URL for Poseidon circuit repeated for 120 times
pub const POSEIDON_WITNESS_M31_URL: &str =
        "https://storage.googleapis.com/expander-compiled-circuits/poseidon-ci/poseidon_120_witness_m31.txt";
// // URL for Poseidon circuit repeated for 120 times
// pub const POSEIDON_CIRCUIT_BN254_URL: &str =
// "https://storage.googleapis.com/expander-compiled-circuits/poseidon-ci/poseidon_120_circuit_bn254.txt";
// // URL for Poseidon circuit repeated for 120 times
// pub const POSEIDON_WITNESS_BN254_URL: &str =
//         "https://storage.googleapis.com/expander-compiled-circuits/poseidon-ci/poseidon_120_witness_bn254.txt";

// NOTE(Hang 08/23/24):
// CI process is unhappy about reqwest as a dependency,
// so we use wget as a backup option.
fn download_and_store(url: &str, file: &str) {
    let download = Command::new("bash")
        .arg("-c")
        .arg(format!("wget {url} -O {file}"))
        .output()
        .expect("Failed to download circuit");

    assert!(download.status.success(), "Circuit download failure")
}

pub fn dev_env_data_setup() {
    fs::create_dir_all(DATA_PREFIX).unwrap();

    download_and_store(KECCAK_CIRCUIT_M31_URL, KECCAK_M31_CIRCUIT);
    download_and_store(KECCAK_CIRCUIT_GF2_URL, KECCAK_GF2_CIRCUIT);
    download_and_store(KECCAK_CIRCUIT_BN254_URL, KECCAK_BN254_CIRCUIT);

    download_and_store(KECCAK_WITNESS_M31_URL, KECCAK_M31_WITNESS);
    download_and_store(KECCAK_WITNESS_GF2_URL, KECCAK_GF2_WITNESS);
    download_and_store(KECCAK_WITNESS_BN254_URL, KECCAK_BN254_WITNESS);

    download_and_store(POSEIDON_CIRCUIT_M31_URL, POSEIDON_M31_CIRCUIT);
    // download_and_store(POSEIDON_CIRCUIT_BN254_URL, POSEIDON_BN254_CIRCUIT);

    download_and_store(POSEIDON_WITNESS_M31_URL, POSEIDON_M31_WITNESS);
    // download_and_store(POSEIDON_WITNESS_BN254_URL, POSEIDON_BN254_WITNESS);
}

#[allow(dead_code)]
fn main() {
    dev_env_data_setup()
}
