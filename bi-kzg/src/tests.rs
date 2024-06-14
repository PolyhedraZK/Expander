use ark_std::test_rng;
use halo2curves::bn256::{Bn256, Fr};

use crate::{
    bi_kzg::BiKZG, pcs::PolynomialCommitmentScheme, poly::lagrange_coefficients,
    util::tensor_product_parallel, BivariatePolynomial,
};

#[test]
fn test_bi_kzg_e2e() {
    let mut rng = test_rng();
    let n = 2;
    let m = 4;
    let srs = BiKZG::<Bn256>::gen_srs_for_testing(&mut rng, n, m);

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
    // let poly = BivariatePolynomial::random(&mut rng, n, m);

    let x = Fr::from(9u64);
    let y = Fr::from(10u64);

    assert_eq!(poly.evaluate(&x, &y), Fr::from(85309u64));

    println!("poly lag coeff: {:?}", poly.lagrange_coeffs());

    let commit = BiKZG::<Bn256>::commit(&srs, &poly);
    let (proof, eval) = BiKZG::<Bn256>::open(&srs, &poly, &(x, y));

    assert!(BiKZG::<Bn256>::verify(
        &srs.into(),
        &commit,
        &(x, y),
        &eval,
        &proof
    ));

    assert!(false)
}

#[test]
fn test_lagrange_coeffs() {
    let roots = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
    let point = Fr::from(4u64);
    let result = lagrange_coefficients(&roots, &point);
    assert_eq!(result[0], Fr::from(1u64));
    assert_eq!(result[1], -Fr::from(3u64));
    assert_eq!(result[2], Fr::from(3u64));
}

#[test]
fn test_tensor_product() {
    let vec1 = vec![Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];
    let vec2 = vec![Fr::from(4u64), Fr::from(5u64), Fr::from(6u64)];
    let result = tensor_product_parallel(&vec1, &vec2);
    assert_eq!(result[0], Fr::from(4u64));
    assert_eq!(result[1], Fr::from(2u64) * Fr::from(4u64));
    assert_eq!(result[2], Fr::from(3u64) * Fr::from(4u64));
    assert_eq!(result[3], Fr::from(5u64));
    assert_eq!(result[4], Fr::from(2u64) * Fr::from(5u64));
    assert_eq!(result[5], Fr::from(3u64) * Fr::from(5u64));
    assert_eq!(result[6], Fr::from(6u64));
    assert_eq!(result[7], Fr::from(2u64) * Fr::from(6u64));
    assert_eq!(result[8], Fr::from(3u64) * Fr::from(6u64));
}
