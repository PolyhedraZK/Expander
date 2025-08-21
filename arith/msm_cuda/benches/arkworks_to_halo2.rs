use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[cfg(feature = "bls12_377")]
use ark_bls12_377::{G1Affine, G2Affine};
#[cfg(feature = "bls12_381")]
use ark_bls12_381::{G1Affine, G2Affine};
#[cfg(feature = "bn254")]
use ark_bn254::G1Affine;

use std::str::FromStr;

use msm_cuda::*;

fn criterion_benchmark(c: &mut Criterion) {
    let bench_npow = std::env::var("BENCH_NPOW").unwrap_or("23".to_string());
    let npoints_npow = i32::from_str(&bench_npow).unwrap();

    let (points, _scalars) =
        util::generate_points_scalars_arkworks::<G1Affine>(1usize << npoints_npow);

    let mut group = c.benchmark_group("CUDA");
    group.sample_size(20);

    let name = format!("single core 2**{}", npoints_npow);
    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = black_box(arkworks_g1_affine_to_halo2(&points.as_slice()));
        })
    });

    let name = format!("rayon 2**{}", npoints_npow);
    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = black_box(arkworks_g1_affine_to_halo2_rayon(&points.as_slice()));
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
