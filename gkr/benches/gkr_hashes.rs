use circuit::Circuit;
use config::{Config, GKRConfig, GKRScheme};
use config_macros::declare_gkr_config;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use gkr::{
    utils::{
        KECCAK_BN254_CIRCUIT, KECCAK_BN254_WITNESS, KECCAK_M31_CIRCUIT, KECCAK_M31_WITNESS,
        POSEIDON_M31_CIRCUIT,
    },
    Prover,
};
use gkr_field_config::{BN254Config, M31ExtConfig, GKRFieldConfig};
use mpi_config::MPIConfig;
use std::hint::black_box;
use transcript::{BytesHashTranscript, SHA256hasher};

#[allow(unused_imports)] // The FieldType import is used in the macro expansion
use gkr_field_config::FieldType;
#[allow(unused_imports)] // The FiatShamirHashType import is used in the macro expansion
use config::FiatShamirHashType;


fn prover_run<Cfg: GKRConfig>(config: &Config<Cfg>, circuit: &mut Circuit<Cfg::FieldConfig>) {
    let mut prover = Prover::new(config);
    prover.prepare_mem(circuit);
    prover.prove(circuit);
}

fn benchmark_setup<Cfg: GKRConfig>(
    scheme: GKRScheme,
    circuit_file: &str,
    witness_file: Option<&str>,
) -> (Config<Cfg>, Circuit<Cfg::FieldConfig>) {
    let config = Config::<Cfg>::new(scheme, MPIConfig::new());
    let mut circuit = Circuit::<Cfg::FieldConfig>::load_circuit(circuit_file);

    if let Some(witness_file) = witness_file {
        circuit.load_witness_file(witness_file);
    } else {
        circuit.set_random_input_for_test();
    }

    (config, circuit)
}

fn criterion_gkr_keccak(c: &mut Criterion) {
    declare_gkr_config!(M31ExtConfigSha2, FieldType::M31, FiatShamirHashType::SHA256);
    declare_gkr_config!(BN254ConfigSha2, FieldType::BN254, FiatShamirHashType::SHA256);

    let (m31_config, mut m31_circuit) = benchmark_setup::<M31ExtConfigSha2>(
        GKRScheme::Vanilla,
        KECCAK_M31_CIRCUIT,
        Some(KECCAK_M31_WITNESS),
    );
    let (bn254_config, mut bn254_circuit) = benchmark_setup::<BN254ConfigSha2>(
        GKRScheme::Vanilla,
        KECCAK_BN254_CIRCUIT,
        Some(KECCAK_BN254_WITNESS),
    );

    let num_keccak_m31 = 2 * <M31ExtConfigSha2 as GKRConfig>::FieldConfig::get_field_pack_size();
    let num_keccak_bn254 = 2 * <BN254ConfigSha2 as GKRConfig>::FieldConfig::get_field_pack_size();

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
                {
                    prover_run(&m31_config, &mut m31_circuit);
                    black_box(())
                };
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
                {
                    prover_run(&bn254_config, &mut bn254_circuit);
                    black_box(())
                };
            })
        },
    );
}

fn criterion_gkr_poseidon(c: &mut Criterion) {
    declare_gkr_config!(M31ExtConfigSha2, FieldType::M31, FiatShamirHashType::SHA256);
    declare_gkr_config!(BN254ConfigSha2, FieldType::BN254, FiatShamirHashType::SHA256);

    let (m31_config, mut m31_circuit) =
        benchmark_setup::<M31ExtConfigSha2>(GKRScheme::GkrSquare, POSEIDON_M31_CIRCUIT, None);

    let mut group = c.benchmark_group("single thread proving poseidon by GKR^2");
    let num_poseidon_m31 = 120 * <M31ExtConfigSha2 as GKRConfig>::FieldConfig::get_field_pack_size();

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
                {
                    prover_run(&m31_config, &mut m31_circuit);
                    black_box(())
                };
            })
        },
    );
}

criterion_group!(benches, criterion_gkr_keccak, criterion_gkr_poseidon);
criterion_main!(benches);
