use ark_std::test_rng;
use halo2curves::bn256::Fr;

use crate::{
    bi_fft_in_place, tensor_product_parallel, BivariateLagrangePolynomial, BivariatePolynomial,
};

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

#[test]
fn test_bivariate_poly_eval() {
    {
        let poly = BivariatePolynomial::new(
            vec![
                Fr::from(1u64),
                Fr::from(2u64),
                Fr::from(3u64),
                Fr::from(4u64),
            ],
            2,
            2,
        );
        let x = Fr::from(5u64);
        let y = Fr::from(7u64);
        let result = poly.evaluate(&x, &y);
        assert_eq!(
            result,
            Fr::from(1u64) + Fr::from(2u64) * x + Fr::from(3u64) * y + Fr::from(4u64) * x * y
        );
    }

    {
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
            2,
            4,
        );
        let x = Fr::from(9u64);
        let y = Fr::from(10u64);
        let result = poly.evaluate(&x, &y);
        assert_eq!(
            result,
            Fr::from(1u64)
                + Fr::from(2u64) * x
                + (Fr::from(3u64) + Fr::from(4u64) * x) * y
                + (Fr::from(5u64) + Fr::from(6u64) * x) * y * y
                + (Fr::from(7u64) + Fr::from(8u64) * x) * y * y * y
        );
    }

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
        4,
        2,
    );
    let x = Fr::from(9u64);
    let y = Fr::from(10u64);
    let result = poly.evaluate(&x, &y);
    assert_eq!(
        result,
        Fr::from(1u64)
            + Fr::from(2u64) * x
            + Fr::from(3u64) * x * x
            + Fr::from(4u64) * x * x * x
            + (Fr::from(5u64)
                + Fr::from(6u64) * x
                + Fr::from(7u64) * x * x
                + Fr::from(8u64) * x * x * x)
                * y
    );
}

#[test]
fn test_eval_at_y() {
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
        2,
        4,
    );
    let eval_at_y = poly.evaluate_at_y(&Fr::from(10u64));
    assert_eq!(eval_at_y, vec![Fr::from(7531u64), Fr::from(8642u64)]);
}

#[test]
fn test_interpolation() {
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
        2,
        4,
    );

    let lagrange_coeffs = poly.interpolate();

    // From sage script
    // poly_lag_coeff = [
    // 0x0000000000000000000000000000000000000000000000000000000000000024,
    // 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffffd,
    // 0x00000000000000059e26bcea0d48bac65a4e1a8be2302529067f891b047e4e50,
    // 0x0000000000000000000000000000000000000000000000000000000000000000,
    // 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffff9,
    // 0x0000000000000000000000000000000000000000000000000000000000000000,
    // 0x30644e72e131a0241a2988cc74389d96cde5cdbc97894b683d626c78eb81b1a1,
    // 0x0000000000000000000000000000000000000000000000000000000000000000]
    assert_eq!(lagrange_coeffs.len(), 8);
    assert_eq!(
        format!("{:?}", lagrange_coeffs[0]),
        "0x0000000000000000000000000000000000000000000000000000000000000024"
    );
    assert_eq!(
        format!("{:?}", lagrange_coeffs[1]),
        "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffffd"
    );
    assert_eq!(
        format!("{:?}", lagrange_coeffs[2]),
        "0x00000000000000059e26bcea0d48bac65a4e1a8be2302529067f891b047e4e50"
    );
    assert_eq!(
        format!("{:?}", lagrange_coeffs[3]),
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(
        format!("{:?}", lagrange_coeffs[4]),
        "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffff9"
    );
    assert_eq!(
        format!("{:?}", lagrange_coeffs[5]),
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(
        format!("{:?}", lagrange_coeffs[6]),
        "0x30644e72e131a0241a2988cc74389d96cde5cdbc97894b683d626c78eb81b1a1"
    );
    assert_eq!(
        format!("{:?}", lagrange_coeffs[7]),
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
}

#[test]
fn test_from_y() {
    let b = Fr::from(10u64);
    let n = 2;
    let m = 4;
    let poly1 = BivariateLagrangePolynomial::from_y_monomial(&b, n, m);
    let poly2 = BivariatePolynomial::new(
        vec![
            -b,
            Fr::from(0u64),
            Fr::from(1u64),
            Fr::from(0u64),
            Fr::from(0u64),
            Fr::from(0u64),
            Fr::from(0u64),
            Fr::from(0u64),
        ],
        n,
        m,
    );
    assert_eq!(poly1.coefficients, poly2.interpolate());
}

#[test]
fn test_bi_fft() {
    {
        let n = 4;
        let m = 4;
        let poly = BivariatePolynomial::new(
            vec![
                Fr::from(1u64),
                Fr::from(2u64),
                Fr::from(4u64),
                Fr::from(8u64),
                Fr::from(16u64),
                Fr::from(32u64),
                Fr::from(64u64),
                Fr::from(128u64),
                Fr::from(256u64),
                Fr::from(128u64),
                Fr::from(64u64),
                Fr::from(32u64),
                Fr::from(16u64),
                Fr::from(8u64),
                Fr::from(4u64),
                Fr::from(2u64),
            ],
            n,
            m,
        );
        let mut poly_lag2 = poly.coefficients.clone();
        let poly_lag = poly.interpolate();
        bi_fft_in_place(&mut poly_lag2, n, m);
        assert_eq!(poly_lag, poly_lag2);
    }

    let mut rng = test_rng();

    for m in [2, 4, 8, 16, 32, 64].iter() {
        for n in [2, 4, 8, 16, 32, 64].iter() {
            let poly = BivariatePolynomial::<Fr>::random(&mut rng, *n, *m);
            let mut poly_lag2 = poly.coefficients.clone();
            let poly_lag = poly.evaluate_at_roots();
            bi_fft_in_place(&mut poly_lag2, *n, *m);
            assert_eq!(poly_lag, poly_lag2);
        }
    }
}
