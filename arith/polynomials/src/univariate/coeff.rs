use arith::{FFTField, Field};
use ark_std::{end_timer, start_timer};
use rand::RngCore;

use crate::{gaussian_elimination, powers_of_field_elements, primitive_root_of_unity};

use super::{best_fft, UnivariatePolynomial};

impl<F: Field> UnivariatePolynomial<F> {
    #[inline]
    pub fn new(coefficients: Vec<F>) -> Self {
        Self { coefficients }
    }

    #[inline]
    pub fn degree(&self) -> usize {
        self.coefficients.len() - 1
    }

    #[inline]
    pub fn random(mut rng: impl RngCore, degree: usize) -> Self {
        let coefficients = (0..degree).map(|_| F::random_unsafe(&mut rng)).collect();
        Self::new(coefficients)
    }

    #[inline]
    /// Compute poly / (x-point) using univariate division
    pub fn quotient_polynomial(&self, point: &F) -> Self {
        Self::new(univariate_quotient(&self.coefficients, point))
    }

    /// evaluate the polynomial at x
    #[inline]
    pub fn evaluate(&self, x: &F) -> F {
        let x_power = powers_of_field_elements(x, self.coefficients.len());
        self.coefficients
            .iter()
            .zip(x_power.iter())
            .fold(F::ZERO, |acc, (c, x_i)| acc + *c * *x_i)
    }

    pub fn from_evals(evals: &[F]) -> Self {
        Self {
            coefficients: Self::vandermonde_interpolation(evals),
        }
    }

    fn vandermonde_interpolation(evals: &[F]) -> Vec<F> {
        let n = evals.len();
        let xs: Vec<F> = (0..n).map(|x| F::from(x as u32)).collect();

        let mut vandermonde: Vec<Vec<F>> = Vec::with_capacity(n);
        for i in 0..n {
            let mut row = Vec::with_capacity(n);
            let x = xs[i];
            row.push(F::one());
            row.push(x);
            for j in 2..n {
                row.push(row[j - 1] * x);
            }
            row.push(evals[i]);
            vandermonde.push(row);
        }

        gaussian_elimination(&mut vandermonde)
    }
}

impl<F: FFTField> UnivariatePolynomial<F> {
    /// interpolate the polynomial over the roots via FFT
    #[inline]
    pub fn interpolate(&self) -> Vec<F> {
        let mut res = self.coefficients.clone();
        let omega = primitive_root_of_unity(self.coefficients.len());
        let log_n = self.coefficients.len().trailing_zeros();
        best_fft(&mut res, omega, log_n);
        res
    }
}

/// Compute poly / (x-point) using univariate division
pub fn univariate_quotient<F: Field>(poly: &[F], point: &F) -> Vec<F> {
    let timer = start_timer!(|| format!("Univariate quotient of degree {}", poly.len() - 1));
    let mut dividend_coeff = poly.to_vec();
    let divisor = [-*point, F::from(1u32)];
    let mut coefficients = vec![];

    let mut dividend_pos = dividend_coeff.len() - 1;
    let divisor_pos = divisor.len() - 1;
    let mut difference = dividend_pos as isize - divisor_pos as isize;

    while difference >= 0 {
        let term_quotient = dividend_coeff[dividend_pos] * divisor[divisor_pos].inv().unwrap();
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
    coefficients.push(F::from(0u32));
    end_timer!(timer);
    coefficients
}
