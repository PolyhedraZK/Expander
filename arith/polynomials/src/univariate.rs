mod fft;
pub use fft::*;

use arith::Field;
use ark_std::{end_timer, start_timer};

/// Compute poly / (x-point) using univariate division
pub fn univariate_quotient<F: Field>(poly: &[F], point: &F) -> Vec<F> {
    let timer = start_timer!(|| format!("Univariate quotient of degree {}", poly.len()));
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
