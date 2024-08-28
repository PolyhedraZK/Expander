use ark_std::{end_timer, log2, start_timer};
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

/// Compute poly / (x-point) using univariate division
pub fn univariate_quotient<F: PrimeField>(poly: &[F], point: &F) -> Vec<F> {
    let timer = start_timer!(|| format!("Univariate quotient of degree {}", poly.len()));
    let mut dividend_coeff = poly.to_vec();
    let divisor = [-*point, F::from(1u64)];
    let mut coefficients = vec![];

    let mut dividend_pos = dividend_coeff.len() - 1;
    let divisor_pos = divisor.len() - 1;
    let mut difference = dividend_pos as isize - divisor_pos as isize;

    while difference >= 0 {
        let term_quotient = dividend_coeff[dividend_pos] * divisor[divisor_pos].invert().unwrap();
        coefficients.push(term_quotient);

        for i in (0..=divisor_pos).rev() {
            let difference = difference as usize;
            let x = divisor[i];
            let y = x * term_quotient;
            let z = dividend_coeff[difference + i];
            dividend_coeff[difference + i] = z - y;
        }

        dividend_pos -= 1;
        difference -= 1;
    }
    coefficients.reverse();
    coefficients.push(F::from(0u64));
    end_timer!(timer);
    coefficients
}
