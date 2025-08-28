// Copyright Supranational LLC
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(feature = "cuda", feature = "bls12_377"))]
use ark_bls12_377::{G1Affine, G2Affine};
#[cfg(all(feature = "cuda", feature = "bls12_381"))]
use ark_bls12_381::{G1Affine, G2Affine};
#[cfg(all(feature = "cuda", feature = "bn254"))]
use ark_bn254::{G1Affine, G2Affine};
#[cfg(feature = "cuda")]
use ark_ec::msm::VariableBaseMSM;
#[cfg(feature = "cuda")]
use ark_ec::ProjectiveCurve;
#[cfg(feature = "cuda")]
use ark_ff::BigInteger256;
#[cfg(feature = "cuda")]
use halo2curves::bn256::Fr;
#[cfg(feature = "cuda")]
use halo2curves::ff::PrimeField as _;

#[cfg(feature = "cuda")]
use std::mem::transmute;
#[cfg(feature = "cuda")]
use std::str::FromStr;

#[cfg(feature = "cuda")]
use msm_cuda::*;

#[cfg(feature = "cuda")]
#[test]
fn msm_correctness_arkworks() {
    let test_npow = std::env::var("TEST_NPOW").unwrap_or("15".to_string());
    let npoints_npow = i32::from_str(&test_npow).unwrap();

    let (points, scalars) =
        util::generate_points_scalars_arkworks::<G1Affine>(1usize << npoints_npow);

    let msm_result = multi_scalar_mult_arkworks(points.as_slice(), unsafe {
        std::mem::transmute::<&[_], &[BigInteger256]>(scalars.as_slice())
    })
    .into_affine();

    let arkworks_result = VariableBaseMSM::multi_scalar_mul(points.as_slice(), unsafe {
        std::mem::transmute::<&[_], &[BigInteger256]>(scalars.as_slice())
    })
    .into_affine();

    assert_eq!(msm_result, arkworks_result);
}

#[cfg(feature = "cuda")]
use halo2curves::msm::best_multiexp;

#[cfg(feature = "cuda")]
#[test]
fn msm_correctness_halo2() {
    let test_npow = std::env::var("TEST_NPOW").unwrap_or("15".to_string());
    let npoints_npow = i32::from_str(&test_npow).unwrap();

    let (points, scalars) =
        util::generate_points_scalars_arkworks::<G1Affine>(1usize << npoints_npow);

    let msm_result_arkworks = multi_scalar_mult_arkworks(points.as_slice(), unsafe {
        std::mem::transmute::<&[_], &[BigInteger256]>(scalars.as_slice())
    })
    .into_affine();

    let points_halo2 = arkworks_g1_affine_to_halo2(&points);
    let scalars_halo2 = scalars
        .iter()
        .map(|s| {
            Fr::from_repr(unsafe { transmute::<_, [u8; 32]>(*s) })
                .expect("Failed to convert scalar")
        })
        .collect::<Vec<_>>();
    let msm_result_halo2 = best_multiexp(&scalars_halo2, &points_halo2).into();

    assert_eq!(
        arkworks_g1_affine_to_halo2(&[msm_result_arkworks]),
        vec![msm_result_halo2]
    );

    let msm_result_gpu = multi_scalar_mult_halo2(points_halo2.as_slice(), &scalars_halo2);

    assert_eq!(msm_result_gpu, msm_result_halo2);
}

#[cfg(all(feature = "cuda", feature = "bn254"))]
#[test]
fn halo2_arkworks_repr() {
    use ark_ec::AffineCurve;
    use halo2curves::bn256::{Fr, G1Affine as G1AffineHalo2};
    use std::mem::transmute;

    let arkworks_fr = <G1Affine as AffineCurve>::ScalarField::from(42u64);
    let halo2_fr = Fr::from(42u64);

    let arkworks_memory_bytes = unsafe { transmute::<_, [u8; 32]>(arkworks_fr) };
    let halo2_memory_bytes = unsafe { transmute::<_, [u8; 32]>(halo2_fr) };

    assert_eq!(arkworks_memory_bytes, halo2_memory_bytes);

    let arkworks_g1 = G1Affine::prime_subgroup_generator();
    let halo2_g1 = G1AffineHalo2::generator();

    let arkworks_memory_bytes = unsafe { transmute::<_, [u8; 64]>([arkworks_g1.x, arkworks_g1.y]) };
    let halo2_memory_bytes = unsafe { transmute::<_, [u8; 64]>([halo2_g1.x, halo2_g1.y]) };

    println!("arkworks_g1_bytes: {:?}", arkworks_memory_bytes);
    println!("halo2_g1_bytes: {:?}", halo2_memory_bytes);

    assert_eq!(arkworks_memory_bytes, halo2_memory_bytes);
}

#[cfg(all(feature = "cuda", any(feature = "bls12_381", feature = "bls12_377", feature = "bn254")))]
#[test]
fn msm_fp2_correctness() {
    let test_npow = std::env::var("TEST_NPOW").unwrap_or("14".to_string());
    let npoints_npow = i32::from_str(&test_npow).unwrap();

    let (points, scalars) =
        util::generate_points_scalars_arkworks::<G2Affine>(1usize << npoints_npow);

    let msm_result = multi_scalar_mult_fp2_arkworks(points.as_slice(), unsafe {
        std::mem::transmute::<&[_], &[BigInteger256]>(scalars.as_slice())
    })
    .into_affine();

    let arkworks_result = VariableBaseMSM::multi_scalar_mul(points.as_slice(), unsafe {
        std::mem::transmute::<&[_], &[BigInteger256]>(scalars.as_slice())
    })
    .into_affine();

    assert_eq!(msm_result, arkworks_result);
}
