use arith::Field;
use criterion::{criterion_group, criterion_main, Criterion};
use mersenne31::{M31x16, M31};
use poseidon::PoseidonParams;

const REPEAT: usize = 1000;

fn bench_poseidon_m31(c: &mut Criterion) {
    c.bench_function("poseidon_m31", |b| {
        let mut rng = rand::thread_rng();
        let param = PoseidonParams::<M31, M31x16>::new();
        let mut state = M31x16::random_unsafe(&mut rng);

        b.iter(|| {
            (0..REPEAT).for_each(|_| param.permute(&mut state));
        });
    });
}

criterion_group!(benches, bench_poseidon_m31);
criterion_main!(benches);
