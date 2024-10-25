use crate::{powers_of_field_elements, primitive_root_of_unity};

use super::{lagrange_coefficients, BivariateLagrangePolynomial, BivariatePolynomial};
use halo2curves::ff::{Field, PrimeField};
use rand::RngCore;

impl<F: Field> BivariateLagrangePolynomial<F> {
    #[inline]
    pub fn new(coeffs: Vec<F>, degree_0: usize, degree_1: usize) -> Self {
        assert_eq!(coeffs.len(), degree_0 * degree_1);
        Self {
            coefficients: coeffs,
            degree_0,
            degree_1,
        }
    }

    #[inline]
    pub fn random(mut rng: impl RngCore, degree_0: usize, degree_1: usize) -> Self {
        let coefficients = (0..degree_0 * degree_1)
            .map(|_| F::random(&mut rng))
            .collect();
        Self::new(coefficients, degree_0, degree_1)
    }
}

impl<F: PrimeField> From<BivariatePolynomial<F>> for BivariateLagrangePolynomial<F> {
    #[inline]
    fn from(poly: BivariatePolynomial<F>) -> Self {
        Self::from(&poly)
    }
}

impl<F: PrimeField> From<&BivariatePolynomial<F>> for BivariateLagrangePolynomial<F> {
    #[inline]
    fn from(poly: &BivariatePolynomial<F>) -> Self {
        let coeffs = poly.interpolate();
        BivariateLagrangePolynomial::new(coeffs, poly.degree_0, poly.degree_1)
    }
}

impl<F: PrimeField> BivariateLagrangePolynomial<F> {
    /// construct a bivariate lagrange polynomial from a monomial f(y) = y - b
    #[inline]
    pub fn from_y_monomial(b: &F, n: usize, m: usize) -> Self {
        // roots of unity for supported_n and supported_m
        let omega_1 = {
            let omega = F::ROOT_OF_UNITY;
            omega.pow_vartime([(1 << F::S) / m as u64])
        };
        let mut coeffs = vec![F::ZERO; n * m];
        for i in 0..m {
            let element = omega_1.pow_vartime([i as u64]) - *b;
            coeffs[i * n..(i + 1) * n].copy_from_slice(vec![element; n].as_slice());
        }
        BivariateLagrangePolynomial::new(coeffs, n, m)
    }
}

impl<F: PrimeField> BivariateLagrangePolynomial<F> {
    /// evaluate the polynomial at (x, y)
    #[inline]
    pub fn evaluate(&self, x: &F, y: &F) -> F {
        let omega_0 = primitive_root_of_unity::<F>(self.degree_0);
        let omega_1 = primitive_root_of_unity::<F>(self.degree_1);

        let powers_of_omega_0 = powers_of_field_elements(&omega_0, self.degree_0);
        let lagrange_x = lagrange_coefficients(&powers_of_omega_0, x);

        let powers_of_omega_1 = powers_of_field_elements(&omega_1, self.degree_1);
        let lagrange_y = lagrange_coefficients(&powers_of_omega_1, y);

        self.coefficients
            .chunks_exact(self.degree_0)
            .zip(lagrange_y.iter())
            .fold(F::ZERO, |acc, (chunk, y_i)| {
                acc + chunk
                    .iter()
                    .zip(lagrange_x.iter())
                    .fold(F::ZERO, |acc, (c, x_i)| acc + *c * *x_i)
                    * y_i
            })
    }

    #[inline]
    pub fn evaluate_at_y(&self, y: &F) -> Vec<F> {
        let omega_1 = primitive_root_of_unity::<F>(self.degree_1);
        let powers_of_omega_1 = powers_of_field_elements(&omega_1, self.degree_1);
        let lagrange_y = lagrange_coefficients(&powers_of_omega_1, y);

        self.coefficients
            .chunks_exact(self.degree_0)
            .zip(lagrange_y.iter())
            .fold(vec![F::ZERO; self.degree_0], |acc, (chunk, y_i)| {
                let mut ret = acc.clone();
                for i in 0..self.degree_0 {
                    ret[i] += chunk[i] * y_i;
                }
                ret
            })
    }
}
