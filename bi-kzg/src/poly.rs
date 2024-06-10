use halo2curves::ff::Field;
use rand::RngCore;

use crate::structs::BivaraitePolynomial;
use crate::util::powers_of_field_elements;

impl<F: Field> BivaraitePolynomial<F> {
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


}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bivariate_poly_eval() {
        use crate::structs::BivaraitePolynomial;
        use halo2curves::bn256::Fr;
        {
            let poly = BivaraitePolynomial::new(
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
            let poly = BivaraitePolynomial::new(
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

        let poly = BivaraitePolynomial::new(
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
}
