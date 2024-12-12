use arith::Field;
use criterion::{criterion_group, criterion_main, Criterion};
use mersenne31::M31x16;
use poseidon::{PoseidonM31Params, PoseidonM31State};

const REPEAT: usize = 1000;

fn bench_poseidon_m31(c: &mut Criterion) {
    c.bench_function("poseidon_m31", |b| {
        let mut rng = rand::thread_rng();
        let param = PoseidonM31Params::new(&mut rand::thread_rng());
        let mut state = PoseidonM31State {
            state: M31x16::random_unsafe(&mut rng),
        };

        b.iter(|| {
            (0..REPEAT).for_each(|_| param.permute(&mut state));
        });
    });
}

criterion_group!(benches, bench_poseidon_m31);
criterion_main!(benches);
