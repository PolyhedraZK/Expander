use arith::Field;

use crate::MultiLinearPoly;

#[derive(Debug, Clone)]
pub struct RefMultiLinearPoly<'ref_life, F: Field> {
    pub coeffs: &'ref_life Vec<F>,
}

impl<'ref_life, 'outer: 'ref_life, F: Field> RefMultiLinearPoly<'ref_life, F> {
    #[inline]
    pub fn from_ref(evals: &'outer Vec<F>) -> Self {
        Self { coeffs: evals }
    }

    pub fn evaluate_with_buffer(&self, point: &[F], scratch: &mut [F]) -> F {
        MultiLinearPoly::evaluate_with_buffer(self.coeffs, point, scratch)
    }
}

#[derive(Debug)]
pub struct MutRefMultiLinearPoly<'ref_life, F: Field> {
    pub coeffs: &'ref_life mut Vec<F>,
}

impl<'ref_life, 'outer_mut: 'ref_life, F: Field> MutRefMultiLinearPoly<'ref_life, F> {
    #[inline]
    pub fn from_ref(evals: &'outer_mut mut Vec<F>) -> Self {
        Self { coeffs: evals }
    }

    pub fn evaluate_with_buffer(&self, point: &[F], scratch: &mut [F]) -> F {
        MultiLinearPoly::evaluate_with_buffer(self.coeffs, point, scratch)
    }

    pub fn fix_top_variable(&mut self, r: F) {
        let n = self.coeffs.len() / 2;
        let (left, right) = self.coeffs.split_at_mut(n);

        left.iter_mut().zip(right.iter()).for_each(|(a, b)| {
            *a += r * (*b - *a);
        });
        self.coeffs.truncate(n);
    }

    pub fn fix_variables(&mut self, partial_point: &[F]) {
        // evaluate single variable of partial point from left to right
        partial_point
            .iter()
            .rev() // need to reverse the order of the point
            .for_each(|point| self.fix_top_variable(*point));
    }
}
