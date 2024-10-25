use arith::Field;
use ark_std::test_rng;
use halo2curves::bn256::Fr;

use crate::{
    lagrange_coefficients, univariate_quotient, UnivariateLagrangePolynomial, UnivariatePolynomial,
};

#[test]
fn test_univariate_division() {
    {
        // x^3 + 1 = (x + 1)(x^2 - x + 1)
        let poly = vec![
            Fr::from(1u64),
            Fr::from(0u64),
            Fr::from(0u64),
            Fr::from(1u64),
        ];
        let point = -Fr::from(1u64);
        let result = univariate_quotient(&poly, &point);
        assert_eq!(
            result,
            vec![
                Fr::from(1u64),
                -Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(0u64)
            ]
        );
    }
    {
        // x^3 - 1 = (x-1)(x^2 + x + 1)
        let poly = vec![
            -Fr::from(1u64),
            Fr::from(0u64),
            Fr::from(0u64),
            Fr::from(1u64),
        ];
        let point = Fr::from(1u64);
        let result = univariate_quotient(&poly, &point);
        assert_eq!(
            result,
            vec![
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(1u64),
                Fr::from(0u64)
            ]
        );
    }
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
fn test_lagrange_transform() {
    let mut rng = test_rng();

    let a = UnivariatePolynomial::<Fr>::random(&mut rng, 4);
    let a_fft = UnivariateLagrangePolynomial::from(&a);

    let x = Fr::random_unsafe(&mut rng);

    let a_eval = a.evaluate(&x);
    let a_fft_eval = a_fft.evaluate(&x);

    assert_eq!(a_eval, a_fft_eval);
}
