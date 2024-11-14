use std::{hint::black_box, ops::Mul, time::Duration};

use arith::{Field, FieldSerde, SimdField};
use ark_std::test_rng;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use gf2::{GF2x128, GF2x64, GF2x8, GF2};
use gf2_128::{GF2_128x8, GF2_128};
use mersenne31::{M31Ext3, M31Ext3x16, M31x16, M31};
use pcs::{OrionPCS, OrionPCSSetup, PolynomialCommitmentScheme, ORION_CODE_PARAMETER_INSTANCE};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};
use tynm::type_name;

fn committing_benchmark_helper<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field + FieldSerde,
    EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    IPPackF: SimdField<Scalar = F>,
    IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
    T: Transcript<EvalF>,
{
    let mut group = c.benchmark_group(format!(
        "Orion PCS committing benchmarking: F = {}, ComPackF = {}",
        type_name::<F>(),
        type_name::<ComPackF>(),
    ));

    let mut rng = test_rng();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let random_poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);

        let orion_pp = OrionPCS::<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>::gen_srs_for_testing(
            &mut rng,
            &OrionPCSSetup {
                num_vars,
                code_parameter: ORION_CODE_PARAMETER_INSTANCE,
            },
        );

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(
                            OrionPCS::<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>::commit(
                                &orion_pp,
                                &random_poly,
                            ),
                        )
                    })
                },
            )
            .sample_size(10)
            .measurement_time(Duration::from_secs(30));
    }
}

fn orion_committing_benchmark(c: &mut Criterion) {
    committing_benchmark_helper::<
        GF2,
        GF2_128,
        GF2x128,
        GF2x8,
        GF2_128x8,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 19, 30);
    committing_benchmark_helper::<
        M31,
        M31Ext3,
        M31x16,
        M31x16,
        M31Ext3x16,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 15, 20);
}

fn opening_benchmark_helper<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field + FieldSerde,
    EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    IPPackF: SimdField<Scalar = F>,
    IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
    T: Transcript<EvalF>,
{
    let mut group = c.benchmark_group(format!(
        "Orion PCS opening benchmarking: F = {}, EvalF = {}, ComPackF = {}",
        type_name::<F>(),
        type_name::<EvalF>(),
        type_name::<ComPackF>(),
    ));

    let mut rng = test_rng();
    let mut transcript = T::new();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let random_poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
        let random_point: Vec<_> = (0..num_vars)
            .map(|_| EvalF::random_unsafe(&mut rng))
            .collect();

        let orion_pp = OrionPCS::<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>::gen_srs_for_testing(
            &mut rng,
            &OrionPCSSetup {
                num_vars,
                code_parameter: ORION_CODE_PARAMETER_INSTANCE,
            },
        );

        let commitment_with_data = OrionPCS::<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>::commit(
            &orion_pp,
            &random_poly,
        );

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(
                            OrionPCS::<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>::open(
                                &orion_pp,
                                &random_poly,
                                &random_point,
                                &commitment_with_data,
                                &mut transcript,
                            ),
                        )
                    })
                },
            )
            .sample_size(10)
            .measurement_time(Duration::from_secs(50));
    }
}

fn orion_opening_benchmark(c: &mut Criterion) {
    opening_benchmark_helper::<
        GF2,
        GF2_128,
        GF2x64,
        GF2x8,
        GF2_128x8,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 19, 30);
    opening_benchmark_helper::<
        M31,
        M31Ext3,
        M31x16,
        M31x16,
        M31Ext3x16,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 15, 20);
}

criterion_group!(bench, orion_committing_benchmark, orion_opening_benchmark);
criterion_main!(bench);
