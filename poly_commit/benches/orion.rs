use std::{hint::black_box, ops::Mul};

use arith::{Field, SimdField};
use ark_std::test_rng;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use gf2::{GF2x128, GF2x8, GF2};
use gf2_128::GF2_128;
use poly_commit::{OrionSRS, OrionScratchPad, ORION_CODE_PARAMETER_INSTANCE};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};
use tynm::type_name;

fn committing_benchmark_helper<F, EvalF, ComPackF, OpenPackF, T>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let mut group = c.benchmark_group(format!(
        "Orion PCS committing benchmarking: F = {}, ComPackF = {}",
        type_name::<F>(),
        type_name::<ComPackF>(),
    ));

    let mut rng = test_rng();
    let mut orion_scratch = OrionScratchPad::default();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);

        let orion_srs =
            OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(
                            orion_srs
                                .commit::<F, ComPackF>(&poly, &mut orion_scratch)
                                .unwrap(),
                        )
                    })
                },
            )
            .sample_size(10);
    }
}

fn orion_committing_benchmark(c: &mut Criterion) {
    committing_benchmark_helper::<
        GF2,
        GF2_128,
        GF2x128,
        GF2x8,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 19, 30);
}

fn opening_benchmark_helper<F, EvalF, ComPackF, OpenPackF, T>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field,
    EvalF: Field + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
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
    let mut orion_scratch = OrionScratchPad::default();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
        let eval_point: Vec<_> = (0..num_vars)
            .map(|_| EvalF::random_unsafe(&mut rng))
            .collect();

        let orion_srs =
            OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

        let _orion_commitment = orion_srs
            .commit::<F, ComPackF>(&poly, &mut orion_scratch)
            .unwrap();

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(orion_srs.open::<F, EvalF, ComPackF, OpenPackF, T>(
                            &poly,
                            &eval_point,
                            &mut transcript,
                            &orion_scratch,
                        ))
                    })
                },
            )
            .sample_size(10);
    }
}

fn orion_opening_benchmark(c: &mut Criterion) {
    opening_benchmark_helper::<GF2, GF2_128, GF2x128, GF2x8, BytesHashTranscript<_, Keccak256hasher>>(
        c, 19, 30,
    );
}

criterion_group!(bench, orion_committing_benchmark, orion_opening_benchmark);
criterion_main!(bench);
