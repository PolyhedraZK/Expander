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
use ark_std::Zero;

#[cfg(feature = "bn254")]
use std::mem::transmute;

#[cfg(feature = "bn254")]
use rayon::prelude::*;

#[cfg(feature = "bn254")]
pub fn halo2_g1_affine_to_arkworks(points: &[G1Affine]) -> Vec<G1AffineArkworks> {
    points
        .par_chunks(1024)
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
    let identity = G1Affine::identity();
    points
        .iter()
        .map(|p| {
            if p.is_zero() {
                identity
            } else {
                unsafe { *(p as *const _ as *const G1Affine) }
            }
        })
        .collect()
}

#[cfg(feature = "bn254")]
pub fn arkworks_g1_affine_to_halo2_rayon(points: &[G1AffineArkworks]) -> Vec<G1Affine> {
    use rayon::prelude::*;
    let identity = G1Affine::identity();
    points
        .par_iter()
        .map(|p| {
            if p.is_zero() {
                identity
            } else {
                // SAFETY: x and y fields are both Fq, which are compatible between libraries
                unsafe { *(p as *const _ as *const G1Affine) }
            }
        })
        .collect()
}

#[cfg(feature = "bn254")]
pub fn multi_scalar_mult_halo2(points: &[G1Affine], scalars: &[Fr]) -> G1Affine {
    use ark_bn254::G1Projective;
    use ark_ec::ProjectiveCurve;
    use utils::timer::Timer;

    #[cfg_attr(feature = "quiet", allow(improper_ctypes))]
    extern "C" {
        fn mult_pippenger_inf_halo2(
            out: *mut G1Projective, /* This G1Projective is supposed to be in its Jacobian
                                     * representation */
            points_with_infinity: *const G1Affine,
            npoints: usize,
            scalars: *const Fr, /* These scalars are supposed to be in their canonical
                                 * representation */
            ffi_affine_sz: usize,
        ) -> sppark::Error;
    }

    let timer = Timer::new("scalars repr transformation", true);
    let scalars_integer = scalars.par_iter().map(|p| p.to_repr()).collect::<Vec<_>>();
    timer.stop();

    let timer = Timer::new("gpu multi-scalar multiplication", true);
    let npoints = points.len();
    if npoints != scalars.len() {
        panic!("length mismatch")
    }

    // We're introducing an arkworks intemediate because halo2 does not use jacobian coordinates
    let mut arkworks_proj = G1Projective::default();
    let err = unsafe {
        mult_pippenger_inf_halo2(
            &mut arkworks_proj as *mut _ as *mut _,
            points.as_ptr() as *const _,
            npoints,
            scalars_integer.as_ptr() as *const _,
            std::mem::size_of::<G1Affine>(),
        )
    };
    let arkworks_affine = arkworks_proj.into_affine();
    if err.code != 0 {
        panic!("{}", String::from(err));
    }

    let halo2_affine = if arkworks_affine.is_zero() {
        G1Affine::identity()
    } else {
        // SAFETY: x and y fields are both Fq, which are compatible between libraries
        unsafe { *(&arkworks_affine as *const _ as *const G1Affine) }
    };

    timer.stop();
    halo2_affine
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
