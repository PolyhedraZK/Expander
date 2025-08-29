#![allow(unused_variables)]

// Dummy implementations for when CUDA is not available

#[cfg(feature = "bn254")]
use ark_bn254::G1Affine as G1AffineArkworks;
use ark_ec::AffineCurve;
use ark_ff::PrimeField;

#[cfg(feature = "bn254")]
use halo2curves::bn256::{Fr, G1Affine};

pub fn multi_scalar_mult_arkworks<G: AffineCurve>(
    points: &[G],
    scalars: &[<G::ScalarField as PrimeField>::BigInt],
) -> G::Projective {
    panic!("CUDA support is not enabled. Please compile with the 'cuda' feature to use GPU acceleration.");
}

#[cfg(any(feature = "bls12_381", feature = "bls12_377", feature = "bn254"))]
pub fn multi_scalar_mult_fp2_arkworks<G: AffineCurve>(
    points: &[G],
    scalars: &[<G::ScalarField as PrimeField>::BigInt],
) -> G::Projective {
    panic!("CUDA support is not enabled. Please compile with the 'cuda' feature to use GPU acceleration.");
}

#[cfg(feature = "bn254")]
pub fn halo2_g1_affine_to_arkworks(points: &[G1Affine]) -> Vec<G1AffineArkworks> {
    panic!("CUDA support is not enabled. Please compile with the 'cuda' feature to use GPU acceleration.");
}

#[cfg(feature = "bn254")]
pub fn arkworks_g1_affine_to_halo2(points: &[G1AffineArkworks]) -> Vec<G1Affine> {
    panic!("CUDA support is not enabled. Please compile with the 'cuda' feature to use GPU acceleration.");
}

#[cfg(feature = "bn254")]
pub fn arkworks_g1_affine_to_halo2_rayon(points: &[G1AffineArkworks]) -> Vec<G1Affine> {
    panic!("CUDA support is not enabled. Please compile with the 'cuda' feature to use GPU acceleration.");
}

#[cfg(feature = "bn254")]
pub fn multi_scalar_mult_halo2(points: &[G1Affine], scalars: &[Fr]) -> G1Affine {
    panic!("CUDA support is not enabled. Please compile with the 'cuda' feature to use GPU acceleration.");
}
