#![allow(clippy::missing_transmute_annotations)]

#[cfg(feature = "bn254")]
use halo2curves::{
    bn256::{Fr, G1Affine},
    ff::PrimeField,
    group::prime::PrimeCurveAffine,
};

#[cfg(feature = "bn254")]
use ark_bn254::G1Affine as G1AffineArkworks;
#[cfg(feature = "bn254")]
use ark_ec::ProjectiveCurve;
#[cfg(feature = "bn254")]
use ark_std::Zero;

#[cfg(feature = "bn254")]
use std::mem::transmute;

#[cfg(feature = "bn254")]
use rayon::prelude::*;

#[cfg(feature = "bn254")]
pub fn halo2_g1_affine_to_arkworks(points: &[G1Affine]) -> Vec<G1AffineArkworks> {
    points
        .par_chunks(1usize << 15)
        .map(|chunk| {
            chunk
                .iter()
                .map(|p| {
                    if p.is_identity().into() {
                        G1AffineArkworks::zero()
                    } else {
                        unsafe {
                            G1AffineArkworks::new(
                                transmute::<_, _>(p.x),
                                transmute::<_, _>(p.y),
                                false,
                            )
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect()
}

#[cfg(feature = "bn254")]
pub fn arkworks_g1_affine_to_halo2(points: &[G1AffineArkworks]) -> Vec<G1Affine> {
    points
        .par_chunks(1usize << 15)
        .map(|chunk| {
            chunk
                .iter()
                .map(|p| {
                    if p.is_zero() {
                        G1Affine::identity()
                    } else {
                        unsafe {
                            G1Affine {
                                x: transmute::<_, _>(p.x),
                                y: transmute::<_, _>(p.y),
                            }
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect()
}

#[cfg(feature = "bn254")]
pub fn multi_scalar_mult_halo2(points: &[G1Affine], scalars: &[Fr]) -> G1Affine {
    use utils::timer::Timer;

    use crate::multi_scalar_mult_arkworks;

    let timer = Timer::new("affine points transformation", true);
    let points_arkworks = halo2_g1_affine_to_arkworks(points);
    timer.stop();

    let timer = Timer::new("scalars repr transformation", true);
    let scalars_integer = scalars
        .par_chunks(1usize << 15)
        .map(|chunk| chunk.iter().map(|p| p.to_repr()).collect::<Vec<_>>())
        .flatten()
        .collect::<Vec<_>>();
    let scalars_arkworks = unsafe {
        std::slice::from_raw_parts(scalars_integer.as_ptr() as *const _, scalars_integer.len())
    };
    timer.stop();

    let timer = Timer::new("gpu multi-scalar multiplication", true);
    let arkworks_result_gpu =
        multi_scalar_mult_arkworks::<G1AffineArkworks>(&points_arkworks, scalars_arkworks)
            .into_affine();
    timer.stop();

    unsafe {
        if arkworks_result_gpu.is_zero() {
            G1Affine::identity()
        } else {
            G1Affine {
                x: transmute::<_, _>(arkworks_result_gpu.x),
                y: transmute::<_, _>(arkworks_result_gpu.y),
            }
        }
    }
}

#[cfg(not(feature = "bn254"))]
pub fn multi_scalar_mult_halo2(points: &[E::G1Affine], scalars: &[E::Fr]) -> E::G1Affine
where
    E: MultiMillerLoop<
        G1 = halo2curves::bn256::G1,
        G2 = halo2curves::bn256::G2,
        G1Affine = halo2curves::bn256::G1Affine,
        G2Affine = halo2curves::bn256::G2Affine,
        Fr = halo2curves::bn256::Fr,
    >,
{
    unimplemented!("halo2 only supports bn254")
}
