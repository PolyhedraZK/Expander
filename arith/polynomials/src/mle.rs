use std::ops::{Index, IndexMut, Mul};

use arith::Field;
use ark_std::log2;

use crate::{EqPolynomial, MultilinearExtension, MutableMultilinearExtension};

#[derive(Debug, Clone, Default)]
pub struct MultiLinearPoly<F: Field> {
    pub coeffs: Vec<F>,
}

impl<F: Field> MultiLinearPoly<F> {
    #[inline]
    pub fn new(evals: Vec<F>) -> Self {
        assert!(evals.len().is_power_of_two());

        Self { coeffs: evals }
    }

    /// Sample a random polynomials.
    #[inline]
    pub fn random(nv: usize, mut rng: impl rand::RngCore) -> Self {
        let coeff = (0..1 << nv).map(|_| F::random_unsafe(&mut rng)).collect();
        Self { coeffs: coeff }
    }

    #[inline]
    pub fn get_num_vars(&self) -> usize {
        log2(self.coeffs.len()) as usize
    }

    /// Evaluate the polynomial at the top variable
    #[inline]
    pub fn fix_top_variable<AF: Field + Mul<F, Output = F>>(&mut self, r: AF) {
        let n = self.coeffs.len() / 2;
        let (left, right) = self.coeffs.split_at_mut(n);

        left.iter_mut().zip(right.iter()).for_each(|(a, b)| {
            *a += r * (*b - *a);
        });
        self.coeffs.truncate(n);
    }

    /// Hyperplonk's implementation
    /// Evaluate the polynomial at a set of variables, from bottom to top
    /// This is equivalent to `evaluate` when partial_point.len() = nv
    #[inline]
    pub fn fix_variables<AF: Field + Mul<F, Output = F>>(&mut self, partial_point: &[AF]) {
        // evaluate single variable of partial point from left to right
        partial_point
            .iter()
            .rev() // need to reverse the order of the point
            .for_each(|point| self.fix_top_variable(*point));
    }

    /// Jolt's implementation
    /// Evaluate the polynomial at a set of variables, from bottom to top
    /// This is equivalent to `evaluate_with_buffer`, but slower
    /// returns Z(r) in O(n) time
    #[inline]
    pub fn evaluate_jolt(&self, r: &[F]) -> F {
        // r must have a value for each variable
        assert_eq!(r.len(), self.get_num_vars());
        let chis = EqPolynomial::evals_jolt(r);
        assert_eq!(chis.len(), self.coeffs.len());
        self.coeffs
            .iter()
            .zip(chis.iter())
            .map(|(c, chi)| *c * *chi)
            .sum()
    }

    #[inline]
    /// Expander's implementation
    /// Generic method to evaluate a multilinear polynomial.
    /// This is the preferred method to evaluate a multilinear polynomial
    /// as it does not require additional memory.
    pub fn evaluate_with_buffer(evals: &[F], point: &[F], scratch: &mut [F]) -> F {
        assert_eq!(1 << point.len(), evals.len());
        assert!(scratch.len() >= evals.len());

        if point.is_empty() {
            evals[0]
        } else {
            for i in 0..(evals.len() >> 1) {
                scratch[i] = (evals[i * 2 + 1] - evals[i * 2]) * point[0] + evals[i * 2];
            }

            let mut cur_eval_size = evals.len() >> 2;
            for r in point.iter().skip(1) {
                for i in 0..cur_eval_size {
                    scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]) * r;
                }
                cur_eval_size >>= 1;
            }
            scratch[0]
        }
    }
}

impl<F: Field> Index<usize> for MultiLinearPoly<F> {
    type Output = F;

    fn index(&self, index: usize) -> &Self::Output {
        &self.coeffs[index]
    }
}

impl<F: Field> MultilinearExtension<F> for MultiLinearPoly<F> {
    fn num_vars(&self) -> usize {
        self.get_num_vars()
    }

    fn hypercube_basis(&self) -> Vec<F> {
        self.coeffs.clone()
    }

    fn hypercube_basis_ref(&self) -> &Vec<F> {
        &self.coeffs
    }

    fn evaluate_with_buffer(&self, point: &[F], scratch: &mut [F]) -> F {
        Self::evaluate_with_buffer(&self.coeffs, point, scratch)
    }
}

impl<F: Field> IndexMut<usize> for MultiLinearPoly<F> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.coeffs[index]
    }
}

impl<F: Field> MutableMultilinearExtension<F> for MultiLinearPoly<F> {
    fn fix_top_variable<AF: Field + std::ops::Mul<F, Output = F>>(&mut self, r: AF) {
        self.fix_top_variable(r)
    }

    fn fix_variables<AF: Field + std::ops::Mul<F, Output = F>>(&mut self, vars: &[AF]) {
        self.fix_variables(vars)
    }
}
