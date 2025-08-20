// Copyright Supranational LLC
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "bn254")]

use criterion::{criterion_group, criterion_main, Criterion};
use halo2curves::bn256::Bn256;

use std::str::FromStr;

use msm_cuda::*;

fn criterion_benchmark(c: &mut Criterion) {
    let bench_npow = std::env::var("BENCH_NPOW").unwrap_or("23".to_string());
    let npoints_npow = i32::from_str(&bench_npow).unwrap();

    let (points, scalars) =
        util::generate_points_scalars_halo2::<Bn256>(1usize << npoints_npow);

    let mut group = c.benchmark_group("CUDA");
    group.sample_size(20);

    let name = format!("2**{}", npoints_npow);
    group.bench_function(name, |b| {
        b.iter(|| {
            let _ = multi_scalar_mult_halo2(&points.as_slice(), &scalars.as_slice());
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
