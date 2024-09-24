// this module benchmarks the performance of different field operations

use arith::{bench_ext_field, bench_field};
use ark_std::test_rng;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use halo2curves::bn256::Fr;
use tynm::type_name;

fn criterion_benchmark(c: &mut Criterion) {
    bench_field::<Fr>(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
