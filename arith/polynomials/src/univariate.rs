use std::ops::{Index, IndexMut};

use arith::{FFTField, Field};

#[derive(Debug, Clone, Default)]
pub struct UnivariatePoly<F: Field> {
    pub coeffs: Vec<F>,
}

#[derive(Debug, Clone, Default)]
pub struct UnivariateLagrangePolynomial<F: FFTField> {
    pub evals: Vec<F>,
}

impl<F: Field> Index<usize> for UnivariatePoly<F> {
    type Output = F;

    fn index(&self, index: usize) -> &Self::Output {
        &self.coeffs[index]
    }
}

impl<F: Field> IndexMut<usize> for UnivariatePoly<F> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.coeffs[index]
    }
}

impl<F: FFTField> Index<usize> for UnivariateLagrangePolynomial<F> {
    type Output = F;

    fn index(&self, index: usize) -> &Self::Output {
        &self.evals[index]
    }
}

impl<F: FFTField> IndexMut<usize> for UnivariateLagrangePolynomial<F> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.evals[index]
    }
}

impl<F: Field> UnivariatePoly<F> {
    #[inline]
    pub fn new(coeffs: Vec<F>) -> Self {
        Self { coeffs }
    }

    // TODO(HS) maybe new ref from ref coeffs, new mut ref from mut ref coeffs?

    /// Sample a random polynomials.
    #[inline]
    pub fn random(degree: usize, mut rng: impl rand::RngCore) -> Self {
        let coeffs = (0..=degree).map(|_| F::random_unsafe(&mut rng)).collect();
        Self { coeffs }
    }

    #[inline]
    pub fn degree(&self) -> usize {
        self.coeffs.len() - 1
    }

    /// Evaluation by Horner's rule
    #[inline]
    pub fn evaluate(&self, point: F) -> F {
        let mut eval = F::ZERO;
        self.coeffs.iter().rev().for_each(|t| {
            eval *= point;
            eval += *t
        });
        eval
    }
}

impl<F: FFTField> UnivariateLagrangePolynomial<F> {
    #[inline]
    pub fn new(evals: Vec<F>) -> Self {
        assert!(evals.len().is_power_of_two());

        Self { evals }
    }

    // TODO(HS) maybe new ref from ref eval, new mut ref from mut ref evals?

    #[inline]
    pub fn random(degree: usize, mut rng: impl rand::RngCore) -> Self {
        assert!((degree + 1).is_power_of_two());
        let evals = (0..=degree).map(|_| F::random_unsafe(&mut rng)).collect();
        Self { evals }
    }

    #[inline]
    pub fn evaluate(&self, point: F) -> F {
        let n = self.evals.len();
        let log_n = n.ilog2() as usize;
        let omega = F::two_adic_generator(log_n);
        let omega_inv = omega.inv().unwrap();

        let nominator_prepare = point.exp(n as u128) - F::ONE;
        let denominator_prepare = {
            let mut omega_i = omega;
            let mut res = F::ONE;

            (1..=n - 1).for_each(|_| {
                res *= F::ONE - omega_i;
                omega_i *= omega;
            });

            res
        };

        let mut omega_i = F::ONE;
        let mut denominator = denominator_prepare;

        self.evals
            .iter()
            .map(|e_i| {
                let nominator_vanisher = point - omega_i;

                let res = if nominator_vanisher.is_zero() {
                    *e_i
                } else {
                    let nominator = nominator_prepare * nominator_vanisher.inv().unwrap();
                    *e_i * nominator * denominator.inv().unwrap()
                };

                omega_i *= omega;
                denominator *= omega_inv;

                res
            })
            .sum()
    }
}

impl<F: FFTField> UnivariatePoly<F> {
    #[inline]
    pub fn fft(mut self) -> UnivariateLagrangePolynomial<F> {
        F::fft_in_place(&mut self.coeffs);

        UnivariateLagrangePolynomial::new(self.coeffs)
    }
}

impl<F: FFTField> UnivariateLagrangePolynomial<F> {
    #[inline]
    pub fn ifft(mut self) -> UnivariatePoly<F> {
        F::ifft_in_place(&mut self.evals);

        UnivariatePoly::new(self.evals)
    }
}

#[cfg(test)]
mod univariate_poly_test {
    use arith::{FFTField, Field};
    use ark_std::test_rng;
    use halo2curves::bn256::Fr;

    use super::UnivariatePoly;

    #[test]
    fn test_univariate_poly_evaluation() {
        let mut rng = test_rng();

        let po2 = 1024;
        let bits = 10;
        let point = Fr::random_unsafe(&mut rng);

        let univariate = UnivariatePoly::random(po2 - 1, &mut rng);
        let evaluation = univariate.evaluate(point);

        let lagrange = univariate.clone().fft();
        let another_evaluatopm = lagrange.evaluate(point);

        assert_eq!(another_evaluatopm, evaluation);

        // NOTE(HS) now we test point being on the smooth multiplicative subgroup
        let omega = Fr::two_adic_generator(bits);
        let omega_i = omega.exp(893 as u128);

        let evaluation = univariate.evaluate(omega_i);
        let another_evaluation = lagrange.evaluate(omega_i);

        assert_eq!(another_evaluation, evaluation);
    }
}
