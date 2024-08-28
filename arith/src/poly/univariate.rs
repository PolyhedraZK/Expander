use ark_std::log2;
use halo2curves::{ff::PrimeField, fft::best_fft};
use rand::RngCore;

use super::powers_of_field_elements;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnivariatePolynomial<F> {
    pub coefficients: Vec<F>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnivariateLagrangePolynomial<F> {
    pub coefficients: Vec<F>,
}

impl<F: PrimeField> UnivariatePolynomial<F> {
    #[inline]
    pub fn new(coefficients: Vec<F>) -> Self {
        Self { coefficients }
    }

    pub fn random(mut rng: impl RngCore, degree: usize) -> Self {
        let coefficients = (0..degree).map(|_| F::random(&mut rng)).collect();
        Self::new(coefficients)
    }

    /// evaluate the polynomial at x
    pub fn evaluate(&self, x: &F) -> F {
        //  todo: parallelization?
        let x_power = powers_of_field_elements(x, self.coefficients.len());
        self.coefficients
            .iter()
            .zip(x_power.iter())
            .map(|(&c, &x_i)| c * x_i)
            .sum()
    }

    pub fn best_fft_in_place(&mut self) {
        let degree = self.coefficients.len();
        let log_degree = log2(degree);
        let mut omega = F::ROOT_OF_UNITY;
        omega = omega.pow_vartime([(1 << F::S) / log_degree as u64]);
        best_fft(&mut self.coefficients, omega, log2(degree))
    }
}
