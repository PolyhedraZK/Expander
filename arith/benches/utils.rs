use arith::bit_reverse;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn bit_reverse_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("bit-reverse");

    let full_seed: usize = 0x123456789ABCDEF;

    for bits in [8, 16, 24, 32] {
        let zeros_on_the_left = 64 - bits;
        let input = (full_seed >> zeros_on_the_left) << zeros_on_the_left;

        group.bench_with_input(
            BenchmarkId::new(format!("benchmark {bits}-bits bit-reverse"), bits),
            &input,
            |b, i| b.iter(|| bit_reverse(*i, bits)),
        );
    }
}

criterion_group!(benches, bit_reverse_benchmark);
criterion_main!(benches);
