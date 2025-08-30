// Copyright Supranational LLC
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(feature = "cuda", feature = "bn254"))]
use criterion::{criterion_group, criterion_main, Criterion};
#[cfg(all(feature = "cuda", feature = "bn254"))]
use halo2curves::{bn256::Bn256, msm::best_multiexp};

#[cfg(all(feature = "cuda", feature = "bn254"))]
use std::str::FromStr;

#[cfg(all(feature = "cuda", feature = "bn254"))]
use msm_cuda::*;

#[cfg(all(feature = "cuda", feature = "bn254"))]
fn criterion_benchmark(c: &mut Criterion) {
    let bench_npow = std::env::var("BENCH_NPOW").unwrap_or("23".to_string());
    let npoints_npow = i32::from_str(&bench_npow).unwrap();

    let (points, scalars) = util::generate_points_scalars_halo2::<Bn256>(1usize << npoints_npow);

    let mut group = c.benchmark_group("CUDA_HALO2");
    group.sample_size(20);

    let name = format!("2**{}", npoints_npow);
    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = multi_scalar_mult_halo2(&points.as_slice(), &scalars.as_slice());
        })
    });

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "bn254"))]
fn criterion_benchmark_2(c: &mut Criterion) {
    let bench_npow = std::env::var("BENCH_NPOW").unwrap_or("23".to_string());
    let npoints_npow = i32::from_str(&bench_npow).unwrap();

    let (points, scalars) = util::generate_points_scalars_halo2::<Bn256>(1usize << npoints_npow);

    let mut group = c.benchmark_group("CPU_HALO2");
    group.sample_size(20);

    let name = format!("2**{}", npoints_npow);
    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = best_multiexp(&scalars.as_slice(), &points.as_slice());
        })
    });

    group.finish();
}

#[cfg(all(feature = "cuda", feature = "bn254"))]
criterion_group!(benches, criterion_benchmark, criterion_benchmark_2);
#[cfg(all(feature = "cuda", feature = "bn254"))]
criterion_main!(benches);

#[cfg(not(all(feature = "cuda", feature = "bn254")))]
fn main() {
    println!("Benchmark requires both 'cuda' and 'bn254' features to be enabled");
}
