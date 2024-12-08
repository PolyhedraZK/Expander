use arith::Field;

use crate::MultiLinearPoly;

#[derive(Debug, Clone)]
pub struct RefMultiLinearPoly<'ref_life, F: Field> {
    pub coeffs: &'ref_life [F],
}

impl<'ref_life, 'outer: 'ref_life, F: Field> RefMultiLinearPoly<'ref_life, F> {
    #[inline]
    pub fn from_ref(evals: &'outer [F]) -> Self {
        Self { coeffs: evals }
    }

    pub fn evaluate_with_buffer(&self, point: &[F], scratch: &mut [F]) -> F {
        MultiLinearPoly::evaluate_with_buffer(self.coeffs, point, scratch)
    }
}

#[derive(Debug)]
pub struct MutRefMultiLinearPoly<'ref_life, F: Field> {
    pub coeffs: &'ref_life mut [F],
}

impl<'ref_life, 'outer_mut: 'ref_life, F: Field> MutRefMultiLinearPoly<'ref_life, F> {
    #[inline]
    pub fn from_ref(evals: &'outer_mut mut [F]) -> Self {
        Self { coeffs: evals }
    }

    pub fn evaluate_with_buffer(&self, point: &[F], scratch: &mut [F]) -> F {
        MultiLinearPoly::evaluate_with_buffer(self.coeffs, point, scratch)
    }
}
