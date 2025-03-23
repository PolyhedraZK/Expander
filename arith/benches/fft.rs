use std::hint::black_box;

use arith::{FFTField, Field, Fr};
use ark_std::test_rng;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn fft_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("FFT");

    const MAX_FFT_SIZE: usize = 1 << 16;
    let mut rng = test_rng();
    let mut bn254_buf: Vec<Fr> = (0..MAX_FFT_SIZE)
        .map(|_| Fr::random_unsafe(&mut rng))
        .collect();

    for group_size_bits in 9..=MAX_FFT_SIZE.ilog2() {
        group.bench_with_input(
            BenchmarkId::new(
                format!("benchmark BN254 {group_size_bits}-bits FFT in place"),
                group_size_bits,
            ),
            &group_size_bits,
            |b, group_size_bits| {
                b.iter(|| {
                    let group_size = 1 << group_size_bits;
                    black_box(Fr::fft_in_place(&mut bn254_buf[..group_size]));
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new(
                format!("benchmark BN254 {group_size_bits}-bits iFFT in place"),
                group_size_bits,
            ),
            &group_size_bits,
            |b, group_size_bits| {
                b.iter(|| {
                    let group_size = 1 << group_size_bits;
                    black_box(Fr::ifft_in_place(&mut bn254_buf[..group_size]));
                })
            },
        );
    }
}

criterion_group!(benches, fft_benchmark);
criterion_main!(benches);
