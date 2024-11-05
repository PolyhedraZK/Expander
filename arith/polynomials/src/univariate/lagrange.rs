use arith::FFTField;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{batch_inversion_in_place, primitive_root_of_unity};

use super::UnivariateLagrangePolynomial;

impl<F: FFTField> UnivariateLagrangePolynomial<F> {
    #[inline]
    pub fn new(coefficients: Vec<F>) -> Self {
        Self { coefficients }
    }

    #[inline]
    // TODO: optimize this code
    pub fn evaluate(&self, x: &F) -> F {
        let n = self.coefficients.len();
        let omega = primitive_root_of_unity::<F>(n);
        let mut result = F::zero();

        // Lagrange interpolation formula
        for i in 0..n {
            let mut term = self.coefficients[i];
            let x_i = omega.exp(i as u128);

            // Compute lagrange basis polynomial
            // TODO: batch inversion
            for j in 0..n {
                if i != j {
                    let x_j = omega.exp(j as u128);
                    term *= (*x - x_j) * (x_i - x_j).inv().unwrap();
                }
            }
            result += term;
        }
        result
    }
}

/// For a point x, compute the coefficients of Lagrange polynomial L_{i}(x) at x, given the roots.
/// `L_{i}(x) = \prod_{j \neq i} \frac{x - r_j}{r_i - r_j}`
///
/// This is a brute-force version of implementation, that has O(n^2) runtime complexity
#[inline]
pub fn lagrange_coefficients<F: FFTField + Send + Sync>(roots: &[F], point: &F) -> Vec<F> {
    let (numerators, mut denominators): (Vec<F>, Vec<F>) = roots
        .par_iter()
        .enumerate()
        .map(|(i, _)| {
            let mut numerator = F::ONE;
            let mut denominator = F::ONE;
            for j in 0..roots.len() {
                if i == j {
                    continue;
                }
                numerator *= *point - roots[j];
                denominator *= roots[i] - roots[j];
            }

            (numerator, denominator)
        })
        .unzip();

    batch_inversion_in_place(&mut denominators);
    numerators
        .iter()
        .zip(denominators.iter())
        .map(|(num, den)| *num * *den)
        .collect()
}
