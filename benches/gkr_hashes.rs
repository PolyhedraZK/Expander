use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use expander_rs::{
    BN254ConfigSha2, Circuit, Config, GKRConfig, GKRScheme, M31ExtConfigSha2, Prover,
};
use std::hint::black_box;

// NOTE(HS): Don't like multiple declarations for circuit files in different files

// circuit for repeating Keccak for 2 times
const KECCAK_CIRCUIT: &str = "data/circuit.txt";
// circuit for repeating Poseidon for 120 times
const POSEIDON_CIRCUIT: &str = "data/poseidon_120_circuit.txt";

fn prover_run<C: GKRConfig>(config: &Config<C>, circuit: &mut Circuit<C>) {
    let mut prover = Prover::new(config);
    prover.prepare_mem(circuit);
    prover.prove(circuit);
}

fn benchmark_setup<C: GKRConfig>(scheme: GKRScheme, circuit_file: &str) -> (Config<C>, Circuit<C>) {
    // wget all necessary files using bash script
    let url_keccak =
        "https://storage.googleapis.com/expander-compiled-circuits/keccak_2_circuit.txt";
    let url_poseidon =
        "https://storage.googleapis.com/expander-compiled-circuits/poseidon_120_circuit.txt";
    let _ = std::process::Command::new("bash")
        .arg("-c")
        .arg("mkdir -p data")
        .output()
        .expect("Failed to create data directory");
    let keccak = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!("wget {} -O data/circuit.txt", url_keccak))
        .output()
        .expect("Failed to download keccak circuit");
    if !keccak.status.success() {
        panic!("Failed to download keccak circuit");
    }
    let _ = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(
            "wget {} -O data/poseidon_120_circuit.txt",
            url_poseidon
        ))
        .output()
        .expect("Failed to download poseidon circuit");

    let config = Config::<C>::new(scheme);
    let mut circuit = Circuit::<C>::load_circuit(circuit_file);
    circuit.set_random_input_for_test();
    (config, circuit)
}

fn criterion_gkr_keccak(c: &mut Criterion) {
    let (m31_config, mut m31_circuit) =
        benchmark_setup::<M31ExtConfigSha2>(GKRScheme::Vanilla, KECCAK_CIRCUIT);
    let (bn254_config, mut bn254_circuit) =
        benchmark_setup::<BN254ConfigSha2>(GKRScheme::Vanilla, KECCAK_CIRCUIT);
    let num_keccak_m31 = 2 * M31ExtConfigSha2::get_field_pack_size();
    let num_keccak_bn254 = 2 * BN254ConfigSha2::get_field_pack_size();

    let mut group = c.benchmark_group("single thread proving keccak by GKR vanilla");
    group.bench_function(
        BenchmarkId::new(
            format!(
                "Over M31, with {} keccak instances per proof",
                num_keccak_m31
            ),
            0,
        ),
        |b| {
            b.iter(|| {
                let _ = black_box(prover_run(&m31_config, &mut m31_circuit));
            })
        },
    );

    group.bench_function(
        BenchmarkId::new(
            format!(
                "Over BN254, with {} keccak instances per proof",
                num_keccak_bn254
            ),
            0,
        ),
        |b| {
            b.iter(|| {
                let _ = black_box(prover_run(&bn254_config, &mut bn254_circuit));
            })
        },
    );
}

fn criterion_gkr_poseidon(c: &mut Criterion) {
    let (m31_config, mut m31_circuit) =
        benchmark_setup::<M31ExtConfigSha2>(GKRScheme::GkrSquare, POSEIDON_CIRCUIT);
    let (bn254_config, mut bn254_circuit) =
        benchmark_setup::<BN254ConfigSha2>(GKRScheme::GkrSquare, POSEIDON_CIRCUIT);

    let mut group = c.benchmark_group("single thread proving poseidon by GKR^2");
    let num_poseidon_m31 = 120 * M31ExtConfigSha2::get_field_pack_size();
    let num_poseidon_bn254 = 120 * BN254ConfigSha2::get_field_pack_size();
    group.bench_function(
        BenchmarkId::new(
            format!(
                "Over M31, with {} poseidon instances per proof",
                num_poseidon_m31
            ),
            0,
        ),
        |b| {
            b.iter(|| {
                let _ = black_box(prover_run(&m31_config, &mut m31_circuit));
            })
        },
    );

    group.bench_function(
        BenchmarkId::new(
            format!(
                "Over BN254, with {} poseidon instances per proof",
                num_poseidon_bn254
            ),
            0,
        ),
        |b| {
            b.iter(|| {
                let _ = black_box(prover_run(&bn254_config, &mut bn254_circuit));
            })
        },
    );
}

criterion_group!(benches, criterion_gkr_keccak, criterion_gkr_poseidon);
criterion_main!(benches);
