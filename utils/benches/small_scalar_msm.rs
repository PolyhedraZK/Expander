use arith::Fr;
use ark_std::rand::{thread_rng, Rng};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use halo2curves::{
    bn256::{G1Affine, G1},
    msm::multiexp_serial,
};
use utils::small_scalar_msm::msm_serial_16bits;

fn generate_test_data(len: usize) -> (Vec<G1Affine>, Vec<Fr>) {
    let mut rng = thread_rng();
    let points = (0..len)
        .map(|_| G1Affine::random(&mut rng))
        .collect::<Vec<_>>();
    let scalars = (0..len)
        .map(|_| Fr::from(rng.gen::<u16>() as u64))
        .collect::<Vec<_>>();
    (points, scalars)
}

fn msm_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("MSM Comparison");

    // Test different input sizes
    for log_len in [8, 10, 12, 14] {
        let len = 1 << log_len;
        let (points, scalars) = generate_test_data(len);

        group.bench_function(format!("msm_serial_16bits_{}", len), |b| {
            b.iter(|| {
                let mut result = G1::default();
                msm_serial_16bits(black_box(&scalars), black_box(&points), &mut result);
                result
            })
        });

        group.bench_function(format!("multiexp_serial_{}", len), |b| {
            b.iter(|| {
                let mut result = G1::default();
                multiexp_serial(black_box(&scalars), black_box(&points), &mut result);
                result
            })
        });
    }

    group.finish();
}

criterion_group!(benches, msm_benchmark);
criterion_main!(benches);
