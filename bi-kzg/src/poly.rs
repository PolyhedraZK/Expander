use halo2curves::ff::Field;
use halo2curves::pairing::Engine;

use crate::structs::BivaraitePolynomial;

impl<F: Field> BivaraitePolynomial<F> {
    pub fn new(coefficients: Vec<F>, degree_0: usize, degree_1: usize) -> Self {
        assert_eq!(coefficients.len(), (degree_0 + 1) * (degree_1 + 1));
        Self {
            coefficients,
            degree_0,
            degree_1,
        }
    }

    pub fn evaluate(&self, x: &F, y: &F) -> F {
        let mut result = F::ZERO;
        let mut x_power = F::ONE;
        for i in 0..=self.degree_0 {
            let mut y_power = F::ONE;
            for j in 0..=self.degree_1 {
                result += self.coefficients[i * (self.degree_1 + 1) + j] * x_power * y_power;
                y_power *= y;
            }
            x_power *= x;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bivariate_poly_eval() {
        use crate::structs::BivaraitePolynomial;
        use halo2curves::bn256::Fr;

        let poly = BivaraitePolynomial::new(
            vec![
                Fr::from(1u64),
                Fr::from(2u64),
                Fr::from(3u64),
                Fr::from(4u64),
            ],
            1,
            1,
        );
        let x = Fr::from(5u64);
        let y = Fr::from(6u64);
        let result = poly.evaluate(&x, &y);
        assert_eq!(
            result,
            Fr::from(1u64) + Fr::from(2u64) * x + Fr::from(3u64) * y + Fr::from(4u64) * x * y
        );
    }
}
