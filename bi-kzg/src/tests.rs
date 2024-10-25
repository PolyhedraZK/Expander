use ark_std::test_rng;
use halo2curves::{
    bn256::{Bn256, Fr},
    ff::Field,
};

use crate::{
    bi_fft::bi_fft_in_place,
    coeff_form_bi_kzg::CoeffFormBiKZG,
    pcs::PolynomialCommitmentScheme,
    poly::{
        lagrange_coefficients, univariate_quotient, BivariateLagrangePolynomial,
        BivariatePolynomial,
    },
    util::tensor_product_parallel,
    BiKZGVerifierParam, LagrangeFormBiKZG,
};

#[test]
fn test_coef_form_bi_kzg_single_pass() {
    let mut rng = test_rng();
    let n = 16;
    let m = 32;

    let srs = CoeffFormBiKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, m);
    let vk = BiKZGVerifierParam::<Bn256>::from(&srs);

    let poly = BivariatePolynomial::<Fr>::random(&mut rng, n, m);

    let x = Fr::random(&mut rng);
    let y = Fr::random(&mut rng);

    let commit = CoeffFormBiKZG::<Bn256>::commit(&srs, &poly);
    let (proof, eval) = CoeffFormBiKZG::<Bn256>::open(&srs, &poly, &(x, y));
    assert!(CoeffFormBiKZG::<Bn256>::verify(
        &vk,
        &commit,
        &(x, y),
        &eval,
        &proof
    ));
}

#[test]
fn test_lagrange_form_bi_kzg_single_pass() {
    let mut rng = test_rng();
    let n = 16;
    let m = 32;

    let srs = LagrangeFormBiKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, m);
    let vk = BiKZGVerifierParam::<Bn256>::from(&srs);

    let poly = BivariateLagrangePolynomial::<Fr>::random(&mut rng, n, m);

    let x = Fr::random(&mut rng);
    let y = Fr::random(&mut rng);

    let commit = LagrangeFormBiKZG::<Bn256>::commit(&srs, &poly);
    let (proof, eval) = LagrangeFormBiKZG::<Bn256>::open(&srs, &poly, &(x, y));
    assert!(CoeffFormBiKZG::<Bn256>::verify(
        &vk,
        &commit,
        &(x, y),
        &eval,
        &proof
    ));
}

#[test]
fn test_coeff_form_bi_kzg_e2e() {
    let mut rng = test_rng();
    let n = 2;
    let m = 4;
    let srs = CoeffFormBiKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, m);
    let vk = BiKZGVerifierParam::<Bn256>::from(&srs);
    let poly = BivariatePolynomial::new(
        vec![
            Fr::from(1u64),
            Fr::from(2u64),
            Fr::from(3u64),
            Fr::from(4u64),
            Fr::from(5u64),
            Fr::from(6u64),
            Fr::from(7u64),
            Fr::from(8u64),
        ],
        n,
        m,
    );

    let x = Fr::from(9u64);
    let y = Fr::from(10u64);

    assert_eq!(poly.evaluate(&x, &y), Fr::from(85309u64));

    let commit = CoeffFormBiKZG::<Bn256>::commit(&srs, &poly);
    let (proof, eval) = CoeffFormBiKZG::<Bn256>::open(&srs, &poly, &(x, y));

    assert!(CoeffFormBiKZG::<Bn256>::verify(
        &vk,
        &commit,
        &(x, y),
        &eval,
        &proof
    ));

    for n in [2, 4, 8, 16] {
        for m in [2, 4, 8, 16] {
            let srs = CoeffFormBiKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, m);
            let vk = BiKZGVerifierParam::<Bn256>::from(&srs);
            for _ in 0..10 {
                let poly = BivariatePolynomial::<Fr>::random(&mut rng, n, m);

                let x = Fr::random(&mut rng);
                let y = Fr::random(&mut rng);

                let commit = CoeffFormBiKZG::<Bn256>::commit(&srs, &poly);
                let (proof, eval) = CoeffFormBiKZG::<Bn256>::open(&srs, &poly, &(x, y));
                assert!(CoeffFormBiKZG::<Bn256>::verify(
                    &vk,
                    &commit,
                    &(x, y),
                    &eval,
                    &proof
                ));
            }
        }
    }
}
