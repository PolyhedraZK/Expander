use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use expander_rs::{BN254Config, Circuit, Config, GKRConfig, GKRScheme, M31ExtConfig, Prover};
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
    let config = Config::<C>::new(scheme);
    let mut circuit = Circuit::<C>::load_circuit(circuit_file);
    circuit.set_random_input_for_test();

    (config, circuit)
}

fn criterion_gkr_keccak(c: &mut Criterion) {
    let (m31_config, mut m31_circuit) =
        benchmark_setup::<M31ExtConfig>(GKRScheme::Vanilla, KECCAK_CIRCUIT);
    let (bn254_config, mut bn254_circuit) =
        benchmark_setup::<BN254Config>(GKRScheme::Vanilla, KECCAK_CIRCUIT);

    let mut group = c.benchmark_group("single thread proving 2 keccak by GKR vanilla");
    group.bench_function(BenchmarkId::new("Over M31", 0), |b| {
        b.iter(|| {
            let _ = black_box(prover_run(&m31_config, &mut m31_circuit));
        })
    });

    group.bench_function(BenchmarkId::new("Over BN254", 0), |b| {
        b.iter(|| {
            let _ = black_box(prover_run(&bn254_config, &mut bn254_circuit));
        })
    });
}

fn criterion_gkr_poseidon(c: &mut Criterion) {
    let (m31_config, mut m31_circuit) =
        benchmark_setup::<M31ExtConfig>(GKRScheme::GkrSquare, POSEIDON_CIRCUIT);
    let (bn254_config, mut bn254_circuit) =
        benchmark_setup::<BN254Config>(GKRScheme::GkrSquare, POSEIDON_CIRCUIT);

    let mut group = c.benchmark_group("single thread proving 120 poseidon by GKR^2");
    group.bench_function(BenchmarkId::new("Over M31", 0), |b| {
        b.iter(|| {
            let _ = black_box(prover_run(&m31_config, &mut m31_circuit));
        })
    });

    group.bench_function(BenchmarkId::new("Over BN254", 0), |b| {
        b.iter(|| {
            let _ = black_box(prover_run(&bn254_config, &mut bn254_circuit));
        })
    });
}

criterion_group!(benches, criterion_gkr_keccak, criterion_gkr_poseidon);
criterion_main!(benches);
