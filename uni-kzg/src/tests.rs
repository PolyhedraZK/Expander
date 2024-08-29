use arith::UnivariatePolynomial;
use ark_std::test_rng;
use halo2curves::{
    bn256::{Bn256, Fr},
    ff::Field,
};

use crate::{CoeffFormUniKZG, PolynomialCommitmentScheme, UniVerifierParam};

#[test]
fn test_uni_kzg_single_pass() {
    let mut rng = test_rng();
    let n = 16;

    let srs = CoeffFormUniKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, 1);
    let vk = UniVerifierParam::<Bn256>::from(&srs);

    let poly = UnivariatePolynomial::<Fr>::random(&mut rng, n);

    let x = Fr::random(&mut rng);

    let commit = CoeffFormUniKZG::<Bn256>::commit(&srs, &poly);
    let (proof, eval) = CoeffFormUniKZG::<Bn256>::open(&srs, &poly, &x);
    assert!(CoeffFormUniKZG::<Bn256>::verify(
        &vk, &commit, &x, &eval, &proof
    ));
}

#[test]
fn test_bi_kzg_e2e() {
    let mut rng = test_rng();

    for n in [2, 4, 8, 16, 32, 64, 128] {
        let srs = CoeffFormUniKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, 1);
        let vk = UniVerifierParam::<Bn256>::from(&srs);
        for _ in 0..10 {
            let poly = UnivariatePolynomial::<Fr>::random(&mut rng, n);

            let x = Fr::random(&mut rng);

            let commit = CoeffFormUniKZG::<Bn256>::commit(&srs, &poly);
            let (proof, eval) = CoeffFormUniKZG::<Bn256>::open(&srs, &poly, &x);
            assert!(CoeffFormUniKZG::<Bn256>::verify(
                &vk, &commit, &x, &eval, &proof
            ));
        }
    }
}
