use halo2curves::ff::{Field, PrimeField};
use rand::RngCore;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::structs::{BivariateLagrangePolynomial, BivariatePolynomial};
use crate::util::powers_of_field_elements;

impl<F: PrimeField> BivariatePolynomial<F> {
    #[inline]
    pub fn new(coefficients: Vec<F>, degree_0: usize, degree_1: usize) -> Self {
        assert_eq!(coefficients.len(), degree_0 * degree_1);
        Self {
            coefficients,
            degree_0,
            degree_1,
        }
    }

    pub fn random(mut rng: impl RngCore, degree_0: usize, degree_1: usize) -> Self {
        let coefficients = (0..degree_0 * degree_1)
            .map(|_| F::random(&mut rng))
            .collect();
        Self::new(coefficients, degree_0, degree_1)
    }

    pub fn evaluate(&self, x: &F, y: &F) -> F {
        let x_power = powers_of_field_elements(x, self.degree_0);
        let y_power = powers_of_field_elements(y, self.degree_1);

        self.coefficients
            .chunks_exact(self.degree_0)
            .zip(y_power.iter())
            .fold(F::ZERO, |acc, (chunk, y_i)| {
                acc + chunk
                    .iter()
                    .zip(x_power.iter())
                    .fold(F::ZERO, |acc, (c, x_i)| acc + *c * *x_i)
                    * y_i
            })
    }

    ///
    pub fn lagrange_coeffs(&self) -> Vec<F> {
        // roots of unity for supported_n and supported_m
        let (omega_0, omega_1) = {
            let omega = F::ROOT_OF_UNITY;
            let omega_0 = omega.pow_vartime(&[(1 << F::S) / self.degree_0 as u64]);
            let omega_1 = omega.pow_vartime(&[(1 << F::S) / self.degree_1 as u64]);

            assert!(
                omega_0.pow_vartime(&[self.degree_0 as u64]) == F::ONE,
                "omega_0 is not root of unity for supported_n"
            );
            assert!(
                omega_1.pow_vartime(&[self.degree_1 as u64]) == F::ONE,
                "omega_1 is not root of unity for supported_m"
            );
            (omega_0, omega_1)
        };
        let powers_of_omega_0 = powers_of_field_elements(&omega_0, self.degree_0);
        let powers_of_omega_1 = powers_of_field_elements(&omega_1, self.degree_1);
        println!(
            "omega len {} {}",
            powers_of_omega_0.len(),
            powers_of_omega_1.len()
        );

        // Todo! Optimize me. This is not efficient.
        let mut res = vec![];
        for omega_1_power in powers_of_omega_1.iter() {
            for omega_0_power in powers_of_omega_0.iter() {
                res.push(self.evaluate(omega_0_power, omega_1_power));
            }
        }

        println!("res: {:?}", res);
        res
    }

    // pub fn from_lagrange_coeffs(coeffs: Vec<F>, degree_0: usize, degree_1: usize) -> Self {
    //     assert_eq!(coeffs.len(), degree_0 * degree_1);
    //     todo!()
    // }
}

/// For x in points, compute the Lagrange coefficients at x given the roots.
/// `L_{i}(x) = \prod_{j \neq i} \frac{x - r_j}{r_i - r_j}``
pub(crate) fn lagrange_coefficients<F: Field + Send + Sync>(roots: &[F], points: &[F]) -> Vec<F> {
    roots
        .par_iter()
        .enumerate()
        .map(|(i, _)| {
            let mut numerator = F::ONE;
            let mut denominator = F::ONE;
            for j in 0..roots.len() {
                if i == j {
                    continue;
                }
                numerator *= roots[j] - points[i];
                denominator *= roots[j] - roots[i];
            }
            numerator * denominator.invert().unwrap()
        })
        .collect()
}

impl<F: Field> BivariateLagrangePolynomial<F> {
    fn new(coeffs: Vec<F>, degree_0: usize, degree_1: usize) -> Self {
        assert_eq!(coeffs.len(), degree_0 * degree_1);
        Self {
            coefficients: coeffs,
            degree_0,
            degree_1,
        }
    }
}

impl<F: PrimeField> From<BivariatePolynomial<F>> for BivariateLagrangePolynomial<F> {
    fn from(poly: BivariatePolynomial<F>) -> Self {
        let coeffs = poly.lagrange_coeffs();
        BivariateLagrangePolynomial::new(coeffs, poly.degree_0, poly.degree_1)
    }
}

impl<F: PrimeField> From<BivariateLagrangePolynomial<F>> for BivariatePolynomial<F> {
    fn from(poly: BivariateLagrangePolynomial<F>) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::structs::BivariatePolynomial;
    use halo2curves::bn256::Fr;

    #[test]
    fn test_bivariate_poly_eval() {
        {
            let poly = BivariatePolynomial::new(
                vec![
                    Fr::from(1u64),
                    Fr::from(2u64),
                    Fr::from(3u64),
                    Fr::from(4u64),
                ],
                2,
                2,
            );
            let x = Fr::from(5u64);
            let y = Fr::from(7u64);
            let result = poly.evaluate(&x, &y);
            assert_eq!(
                result,
                Fr::from(1u64) + Fr::from(2u64) * x + Fr::from(3u64) * y + Fr::from(4u64) * x * y
            );
        }

        {
            let poly = BivariatePolynomial::new(
                vec![
                    Fr::from(1u64),
                    Fr::from(2u64),
                    Fr::from(3u64),
                    Fr::from(4u64),
                    Fr::from(5u64),
                    Fr::from(6u64),
                    Fr::from(7u64),
                    Fr::from(8u64),
                ],
                2,
                4,
            );
            let x = Fr::from(9u64);
            let y = Fr::from(10u64);
            let result = poly.evaluate(&x, &y);
            assert_eq!(
                result,
                Fr::from(1u64)
                    + Fr::from(2u64) * x
                    + (Fr::from(3u64) + Fr::from(4u64) * x) * y
                    + (Fr::from(5u64) + Fr::from(6u64) * x) * y * y
                    + (Fr::from(7u64) + Fr::from(8u64) * x) * y * y * y
            );
        }

        let poly = BivariatePolynomial::new(
            vec![
                Fr::from(1u64),
                Fr::from(2u64),
                Fr::from(3u64),
                Fr::from(4u64),
                Fr::from(5u64),
                Fr::from(6u64),
                Fr::from(7u64),
                Fr::from(8u64),
            ],
            4,
            2,
        );
        let x = Fr::from(9u64);
        let y = Fr::from(10u64);
        let result = poly.evaluate(&x, &y);
        assert_eq!(
            result,
            Fr::from(1u64)
                + Fr::from(2u64) * x
                + Fr::from(3u64) * x * x
                + Fr::from(4u64) * x * x * x
                + (Fr::from(5u64)
                    + Fr::from(6u64) * x
                    + Fr::from(7u64) * x * x
                    + Fr::from(8u64) * x * x * x)
                    * y
        );
    }

    // #[test]
    // fn test_lagrange_coeffs() {
    //     let poly = BivariatePolynomial::new(
    //         vec![
    //             Fr::from(1u64),
    //             Fr::from(2u64),
    //             Fr::from(3u64),
    //             Fr::from(4u64),
    //             Fr::from(5u64),
    //             Fr::from(10u64),
    //             Fr::from(15u64),
    //             Fr::from(20u64),
    //         ],
    //         4,
    //         2,
    //     );

    //     let lagrange_coeffs = poly.lagrange_coeffs();
    //     println!("lag: {:?}", lagrange_coeffs);
    //     assert_eq!(lagrange_coeffs.len(), 8);
    //     assert_eq!(lagrange_coeffs[0], Fr::from(1u64));
    //     assert_eq!(lagrange_coeffs[1], Fr::from(2u64));
    //     assert_eq!(lagrange_coeffs[2], Fr::from(3u64));
    //     assert_eq!(lagrange_coeffs[3], Fr::from(4u64));
    // }
}
