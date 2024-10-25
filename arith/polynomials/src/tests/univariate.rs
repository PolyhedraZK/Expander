use halo2curves::bn256::Fr;

use crate::{lagrange_coefficients, univariate_quotient};

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
