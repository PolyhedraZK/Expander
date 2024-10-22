pub mod coef_form;
pub use coef_form::*;

pub mod lagrange_form;
pub use lagrange_form::*;

pub mod utils;
pub use utils::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BivariatePolynomial<F> {
    pub coefficients: Vec<F>,
    pub degree_0: usize,
    pub degree_1: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BivariateLagrangePolynomial<F> {
    pub coefficients: Vec<F>,
    pub degree_0: usize,
    pub degree_1: usize,
}

// /// For a point x, compute the coefficients of Lagrange polynomial L_{i}(x) at x, given the roots.
// /// `L_{i}(x) = \prod_{j \neq i} \frac{x - r_j}{r_i - r_j}`
// pub(crate) fn lagrange_coefficients<F: Field + Send + Sync>(roots: &[F], points: &F) -> Vec<F> {
//     roots
//         .par_iter()
//         .enumerate()
//         .map(|(i, _)| {
//             let mut numerator = F::ONE;
//             let mut denominator = F::ONE;
//             for j in 0..roots.len() {
//                 if i == j {
//                     continue;
//                 }
//                 numerator *= roots[j] - points;
//                 denominator *= roots[j] - roots[i];
//             }
//             numerator * denominator.invert().unwrap()
//         })
//         .collect()
// }

// /// Compute poly / (x-point) using univariate division
// pub(crate) fn univariate_quotient<F: PrimeField>(poly: &[F], point: &F) -> Vec<F> {
//     let timer = start_timer!(|| format!("Univariate quotient of degree {}", poly.len()));
//     let mut dividend_coeff = poly.to_vec();
//     let divisor = [-*point, F::from(1u64)];
//     let mut coefficients = vec![];

//     let mut dividend_pos = dividend_coeff.len() - 1;
//     let divisor_pos = divisor.len() - 1;
//     let mut difference = dividend_pos as isize - divisor_pos as isize;

//     while difference >= 0 {
//         let term_quotient = dividend_coeff[dividend_pos] * divisor[divisor_pos].invert().unwrap();
//         coefficients.push(term_quotient);

//         for i in (0..=divisor_pos).rev() {
//             let difference = difference as usize;
//             let x = divisor[i];
//             let y = x * term_quotient;
//             let z = dividend_coeff[difference + i];
//             dividend_coeff[difference + i] = z - y;
//         }

//         dividend_pos -= 1;
//         difference -= 1;
//     }
//     coefficients.reverse();
//     coefficients.push(F::from(0u64));
//     end_timer!(timer);
//     coefficients
// }

// impl<F: Field> BivariateLagrangePolynomial<F> {
//     fn new(coeffs: Vec<F>, degree_0: usize, degree_1: usize) -> Self {
//         assert_eq!(coeffs.len(), degree_0 * degree_1);
//         Self {
//             coefficients: coeffs,
//             degree_0,
//             degree_1,
//         }
//     }
// }

// impl<F: PrimeField> From<BivariatePolynomial<F>> for BivariateLagrangePolynomial<F> {
//     fn from(poly: BivariatePolynomial<F>) -> Self {
//         Self::from(&poly)
//     }
// }

// impl<F: PrimeField> From<&BivariatePolynomial<F>> for BivariateLagrangePolynomial<F> {
//     fn from(poly: &BivariatePolynomial<F>) -> Self {
//         let coeffs = poly.interpolate();
//         BivariateLagrangePolynomial::new(coeffs, poly.degree_0, poly.degree_1)
//     }
// }

// impl<F: PrimeField> BivariateLagrangePolynomial<F> {
//     /// construct a bivariate lagrange polynomial from a monomial f(y) = y - b
//     pub(crate) fn from_y_monomial(b: &F, n: usize, m: usize) -> Self {
//         // roots of unity for supported_n and supported_m
//         let omega_1 = {
//             let omega = F::ROOT_OF_UNITY;
//             omega.pow_vartime([(1 << F::S) / m as u64])
//         };
//         let mut coeffs = vec![F::ZERO; n * m];
//         for i in 0..m {
//             let element = omega_1.pow_vartime([i as u64]) - *b;
//             coeffs[i * n..(i + 1) * n].copy_from_slice(vec![element; n].as_slice());
//         }
//         BivariateLagrangePolynomial::new(coeffs, n, m)
//     }
// }
