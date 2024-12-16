use arith::Field;
use criterion::{criterion_group, criterion_main, Criterion};
use hasher::{FieldHasher, FieldHasherState, PoseidonM31x16Ext3, PoseidonParams};
use mersenne31::{M31Ext3, M31};

const REPEAT: usize = 1000;

fn bench_poseidon_m31(c: &mut Criterion) {
    c.bench_function("poseidon_m31", |b| {
        let mut rng = rand::thread_rng();
        let param = PoseidonParams::<M31, M31Ext3, PoseidonM31x16Ext3>::new();
        let state_elems: Vec<_> = (0..PoseidonM31x16Ext3::STATE_WIDTH)
            .map(|_| M31::random_unsafe(&mut rng))
            .collect();
        let mut state = PoseidonM31x16Ext3::from_elems(&state_elems);

        b.iter(|| {
            (0..REPEAT).for_each(|_| param.permute(&mut state));
        });
    });
}

criterion_group!(benches, bench_poseidon_m31);
criterion_main!(benches);
