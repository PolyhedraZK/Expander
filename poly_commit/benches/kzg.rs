use std::hint::black_box;

use arith::{Field, Fr};
use ark_std::test_rng;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use halo2curves::bn256::Bn256;
use poly_commit::{HyperKZGPCS, PolynomialCommitmentScheme};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

fn hyperkzg_committing_benchmark_helper(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) {
    let mut group = c.benchmark_group("HyperKZG PCS committing");

    let mut rng = test_rng();
    let mut scratch_pad = ();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);

        let srs =
            HyperKZGPCS::<Bn256, BytesHashTranscript<Keccak256hasher>>::gen_srs_for_testing(
                &num_vars, &mut rng,
            );

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(
                            HyperKZGPCS::<Bn256, BytesHashTranscript<Keccak256hasher>>::commit(
                                &num_vars,
                                &srs,
                                &poly,
                                &mut scratch_pad,
                            ),
                        )
                    })
                },
            )
            .sample_size(10);
    }
}

fn hyperkzg_committing_benchmark(c: &mut Criterion) {
    hyperkzg_committing_benchmark_helper(c, 8, 15)
}

fn hyperkzg_opening_benchmark_helper(
    c: &mut Criterion,
    lowest_num_vars: usize,
    highest_num_vars: usize,
) {
    let mut group = c.benchmark_group("HyperKZG PCS opening");

    let mut rng = test_rng();
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let mut scratch_pad = ();

    for num_vars in lowest_num_vars..=highest_num_vars {
        let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);

        let srs =
            HyperKZGPCS::<Bn256, BytesHashTranscript<Keccak256hasher>>::gen_srs_for_testing(
                &num_vars, &mut rng,
            );
        let eval_point: Vec<_> = (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect();

        let _ = HyperKZGPCS::<Bn256, BytesHashTranscript<Keccak256hasher>>::commit(
            &num_vars,
            &srs,
            &poly,
            &mut scratch_pad,
        );

        group
            .bench_function(
                BenchmarkId::new(format!("{num_vars} variables"), num_vars),
                |b| {
                    b.iter(|| {
                        _ = black_box(HyperKZGPCS::<Bn256, _>::open(
                            &num_vars,
                            &srs,
                            &poly,
                            &eval_point,
                            &scratch_pad,
                            &mut transcript,
                        ))
                    })
                },
            )
            .sample_size(10);
    }
}

fn hyperkzg_opening_benchmark(c: &mut Criterion) {
    hyperkzg_opening_benchmark_helper(c, 8, 15)
}

criterion_group!(
    bench,
    hyperkzg_committing_benchmark,
    hyperkzg_opening_benchmark
);
criterion_main!(bench);
