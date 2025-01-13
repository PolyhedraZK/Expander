use std::hint::black_box;

use arith::{ExtensionField, Field, SimdField};
use ark_std::test_rng;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use gf2::{GF2x128, GF2x8, GF2};
use gf2_128::GF2_128;
use mersenne31::{M31Ext3, M31x16, M31};
use poly_commit::*;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};
use tynm::type_name;

fn base_field_committing_benchmark_helper<F, ComPackF>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field,
    ComPackF: SimdField<Scalar = F>,
{
    let mut group = c.benchmark_group(format!(
        "Orion PCS base field committing: F = {}, ComPackF = {}",
        type_name::<F>(),
        type_name::<ComPackF>(),
    ));

    let mut rng = test_rng();
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);

        let srs = OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(
                            orion_commit_base_field(&srs, &poly, &mut scratch_pad).unwrap(),
                        )
                    })
                },
            )
            .sample_size(10);
    }
}

fn orion_base_field_committing_benchmark(c: &mut Criterion) {
    base_field_committing_benchmark_helper::<GF2, GF2x128>(c, 19, 30);
    base_field_committing_benchmark_helper::<M31, M31x16>(c, 19, 27);
}

fn simd_field_committing_benchmark_helper<F, SimdF, ComPackF>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let mut group = c.benchmark_group(format!(
        "Orion PCS SIMD field committing: F = {}, SIMD-F = {}, ComPackF = {}",
        type_name::<F>(),
        type_name::<SimdF>(),
        type_name::<ComPackF>(),
    ));

    let mut rng = test_rng();
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let packed_num_vars = num_vars - SimdF::PACK_SIZE.ilog2() as usize;
        let poly = MultiLinearPoly::<SimdF>::random(packed_num_vars, &mut rng);

        let srs = OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

        group
            .bench_function(
                BenchmarkId::new(
                    format!(
                        "{num_vars} variables with {packed_num_vars} being packed poly num vars"
                    ),
                    num_vars,
                ),
                |b| {
                    b.iter(|| {
                        _ = black_box(
                            orion_commit_simd_field(&srs, &poly, &mut scratch_pad).unwrap(),
                        )
                    })
                },
            )
            .sample_size(10);
    }
}

fn orion_simd_field_committing_benchmark(c: &mut Criterion) {
    simd_field_committing_benchmark_helper::<GF2, GF2x8, GF2x128>(c, 19, 32);
    simd_field_committing_benchmark_helper::<M31, M31x16, M31x16>(c, 19, 27);
}

fn base_field_opening_benchmark_helper<F, EvalF, ComPackF, OpenPackF, T>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    OpenPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let mut group = c.benchmark_group(format!(
        "Orion PCS base field opening: F = {}, EvalF = {}, ComPackF = {}, OpenPackF = {}",
        type_name::<F>(),
        type_name::<EvalF>(),
        type_name::<ComPackF>(),
        type_name::<OpenPackF>(),
    ));

    let mut rng = test_rng();
    let mut transcript = T::new();
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let poly = MultiLinearPoly::<F>::random(num_vars, &mut rng);
        let eval_point: Vec<_> = (0..num_vars)
            .map(|_| EvalF::random_unsafe(&mut rng))
            .collect();

        let srs = OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

        let _commitment = orion_commit_base_field(&srs, &poly, &mut scratch_pad).unwrap();

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(orion_open_base_field::<F, EvalF, ComPackF, OpenPackF, T>(
                            &srs,
                            &poly,
                            &eval_point,
                            &mut transcript,
                            &scratch_pad,
                        ))
                    })
                },
            )
            .sample_size(10);
    }
}

fn orion_base_field_opening_benchmark(c: &mut Criterion) {
    base_field_opening_benchmark_helper::<
        GF2,
        GF2_128,
        GF2x128,
        GF2x8,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 19, 30);
    base_field_opening_benchmark_helper::<
        M31,
        M31Ext3,
        M31x16,
        M31x16,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 19, 26);
}

fn simd_field_opening_benchmark_helper<F, SimdF, EvalF, ComPackF, T>(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let mut group = c.benchmark_group(format!(
        "Orion PCS SIMD field opening: SIMD-F = {}, EvalF = {}, ComPackF = {}",
        type_name::<SimdF>(),
        type_name::<EvalF>(),
        type_name::<ComPackF>(),
    ));

    let mut rng = test_rng();
    let mut transcript = T::new();
    let mut scratch_pad = OrionScratchPad::<F, ComPackF>::default();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let packed_num_vars = num_vars - SimdF::PACK_SIZE.ilog2() as usize;
        let poly = MultiLinearPoly::<SimdF>::random(packed_num_vars, &mut rng);
        let eval_point: Vec<_> = (0..num_vars)
            .map(|_| EvalF::random_unsafe(&mut rng))
            .collect();

        let srs = OrionSRS::from_random::<F>(num_vars, ORION_CODE_PARAMETER_INSTANCE, &mut rng);

        let _commitment = orion_commit_simd_field(&srs, &poly, &mut scratch_pad).unwrap();

        group
            .bench_function(
                BenchmarkId::new(
                    format!(
                        "{num_vars} variables with {packed_num_vars} being packed poly num vars"
                    ),
                    num_vars,
                ),
                |b| {
                    b.iter(|| {
                        _ = black_box(orion_open_simd_field::<F, SimdF, _, ComPackF, T>(
                            &srs,
                            &poly,
                            &eval_point,
                            &mut transcript,
                            &scratch_pad,
                        ))
                    })
                },
            )
            .sample_size(10);
    }
}

fn orion_simd_field_opening_benchmark(c: &mut Criterion) {
    simd_field_opening_benchmark_helper::<
        GF2,
        GF2x8,
        GF2_128,
        GF2x128,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 19, 32);
    simd_field_opening_benchmark_helper::<
        M31,
        M31x16,
        M31Ext3,
        M31x16,
        BytesHashTranscript<_, Keccak256hasher>,
    >(c, 19, 27);
}

criterion_group!(
    bench,
    orion_base_field_committing_benchmark,
    orion_base_field_opening_benchmark,
    orion_simd_field_committing_benchmark,
    orion_simd_field_opening_benchmark,
);
criterion_main!(bench);
