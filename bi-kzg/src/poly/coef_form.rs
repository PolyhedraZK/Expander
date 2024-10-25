use ark_std::{end_timer, start_timer};
use halo2curves::ff::PrimeField;
use itertools::Itertools;
use rand::RngCore;

use crate::bi_fft::bi_fft_in_place;
use crate::util::powers_of_field_elements;

use super::BivariatePolynomial;

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

    #[inline]
    pub fn random(mut rng: impl RngCore, degree_0: usize, degree_1: usize) -> Self {
        let coefficients = (0..degree_0 * degree_1)
            .map(|_| F::random(&mut rng))
            .collect();
        Self::new(coefficients, degree_0, degree_1)
    }

    /// evaluate the polynomial at (x, y)
    #[inline]
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

    /// evaluate the polynomial at y, return a univariate polynomial in x
    #[inline]
    pub fn evaluate_at_y(&self, y: &F) -> Vec<F> {
        let mut f_x_b = self.coefficients[0..self.degree_0].to_vec();
        let powers_of_b = powers_of_field_elements(y, self.degree_1);
        powers_of_b
            .iter()
            .zip_eq(self.coefficients.chunks_exact(self.degree_0))
            .skip(1)
            .for_each(|(bi, chunk_i)| {
                f_x_b
                    .iter_mut()
                    .zip(chunk_i.iter())
                    .for_each(|(f, c)| *f += *c * *bi)
            });

        f_x_b
    }

    /// same as interpolate but slower.
    #[inline]
    pub fn evaluate_at_roots(&self) -> Vec<F> {
        let timer = start_timer!(|| format!(
            "Lagrange coefficients of degree {} {}",
            self.degree_0, self.degree_1
        ));

        // roots of unity for supported_n and supported_m
        let (omega_0, omega_1) = {
            let omega = F::ROOT_OF_UNITY;
            let omega_0 = omega.pow_vartime([(1 << F::S) / self.degree_0 as u64]);
            let omega_1 = omega.pow_vartime([(1 << F::S) / self.degree_1 as u64]);

            assert!(
                omega_0.pow_vartime([self.degree_0 as u64]) == F::ONE,
                "omega_0 is not root of unity for supported_n"
            );
            assert!(
                omega_1.pow_vartime([self.degree_1 as u64]) == F::ONE,
                "omega_1 is not root of unity for supported_m"
            );
            (omega_0, omega_1)
        };
        let powers_of_omega_0 = powers_of_field_elements(&omega_0, self.degree_0);
        let powers_of_omega_1 = powers_of_field_elements(&omega_1, self.degree_1);

        let mut res = vec![];
        for omega_1_power in powers_of_omega_1.iter() {
            for omega_0_power in powers_of_omega_0.iter() {
                res.push(self.evaluate(omega_0_power, omega_1_power));
            }
        }
        end_timer!(timer);
        res
    }

    /// interpolate the polynomial over the roots via bi-variate FFT
    #[inline]
    pub fn interpolate(&self) -> Vec<F> {
        let timer = start_timer!(|| format!(
            "Lagrange coefficients of degree {} {}",
            self.degree_0, self.degree_1
        ));

        let mut coeff = self.coefficients.clone();
        bi_fft_in_place(&mut coeff, self.degree_0, self.degree_1);
        end_timer!(timer);
        coeff
    }
}
