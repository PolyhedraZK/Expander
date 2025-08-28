// Copyright Supranational LLC
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

use halo2curves::ff::Field;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::{Curve, Group};
use halo2curves::pairing::MultiMillerLoop;
use num_bigint::BigUint;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;

use ark_ec::{AffineCurve, ProjectiveCurve};
use ark_std::UniformRand;

pub fn generate_points_scalars_arkworks<G: AffineCurve>(
    len: usize,
) -> (Vec<G>, Vec<G::ScalarField>) {
    let rand_gen: usize = std::cmp::min(1usize << 11, len);
    let mut rng = ChaCha20Rng::from_entropy();

    let mut points = <G::Projective as ProjectiveCurve>::batch_normalization_into_affine(
        &(0..rand_gen)
            .map(|_| G::Projective::rand(&mut rng))
            .collect::<Vec<_>>(),
    );
    // Sprinkle in some infinity points
    if len > 2 {
        points[3] = G::zero();
    }
    let scalars = (0..len)
        .map(|_| G::ScalarField::from(BigUint::from(rng.next_u64())))
        .collect::<Vec<_>>();

    while points.len() < len {
        points.append(&mut points.clone());
    }

    points.truncate(len);

    (points, scalars)
}

pub fn generate_points_scalars_halo2<E: MultiMillerLoop>(
    len: usize,
) -> (Vec<E::G1Affine>, Vec<E::Fr>) {
    let rand_gen: usize = std::cmp::min(1usize << 11, len);
    let mut rng = ChaCha20Rng::from_entropy();

    let points_projective = (0..rand_gen)
        .map(|_| E::G1::random(&mut rng))
        .collect::<Vec<_>>();
    let mut points_affine = vec![E::G1Affine::identity(); rand_gen];
    E::G1::batch_normalize(&points_projective, &mut points_affine);

    // Sprinkle in some infinity points
    if len > 2 {
        points_affine[3] = E::G1Affine::identity();
    }
    let scalars = (0..len)
        .map(|_| E::Fr::random(&mut rng))
        .collect::<Vec<_>>();

    while points_affine.len() < len {
        points_affine.append(&mut points_affine.clone());
    }

    points_affine.truncate(len);

    (points_affine, scalars)
}
