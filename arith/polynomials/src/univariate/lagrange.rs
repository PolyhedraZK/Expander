use arith::FFTField;

use crate::primitive_root_of_unity;

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
