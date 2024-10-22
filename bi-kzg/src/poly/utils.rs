use ark_std::{end_timer, start_timer};
use halo2curves::ff::{Field, PrimeField};

/// For a point x, compute the coefficients of Lagrange polynomial L_{i}(x) at x, given the roots.
/// `L_{i}(x) = \prod_{j \neq i} \frac{x - r_j}{r_i - r_j}`
pub fn lagrange_coefficients<F: Field + Send + Sync>(roots: &[F], points: &F) -> Vec<F> {
    roots
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let mut numerator = F::ONE;
            let mut denominator = F::ONE;
            for j in 0..roots.len() {
                if i == j {
                    continue;
                }
                numerator *= roots[j] - points;
                denominator *= roots[j] - roots[i];
            }
            numerator * denominator.invert().unwrap()
        })
        .collect()
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
