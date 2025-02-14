use arith::{BN254Fr, Field};
use halo2curves::{
    bn256::{G1Affine, G1},
    group::Curve,
    msm,
};

pub struct PedersenParams(pub Vec<G1Affine>);

#[allow(unused)]
pub(crate) fn pedersen_setup(length: usize, mut rng: impl rand::RngCore) -> PedersenParams {
    let bases: Vec<G1Affine> = (0..length)
        .map(|_| {
            let temp = BN254Fr::random_unsafe(&mut rng);
            (G1Affine::generator() * temp).to_affine()
        })
        .collect();

    PedersenParams(bases)
}

#[allow(unused)]
pub(crate) fn pedersen_vector_commit(params: &PedersenParams, coeffs: &[BN254Fr]) -> G1Affine {
    let mut what = G1::default();

    msm::multiexp_serial(coeffs, &params.0, &mut what);

    what.to_affine()
}
