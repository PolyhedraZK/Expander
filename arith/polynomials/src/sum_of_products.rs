use arith::Field;

use crate::{MultiLinearPoly, MultilinearExtension};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// A special form of a multi-linear polynomial: f = f0*g0 + f1*g1 + ...
/// where f0, f1, ...  and g0, g1, ... are multi-linear polynomials
/// The sumcheck over this polynomial has a degree of 2
pub struct SumOfProductsPoly<F: Field> {
    /// The list of multi-linear polynomials to be summed
    pub f_and_g_pairs: Vec<(MultiLinearPoly<F>, MultiLinearPoly<F>)>,
}

impl<F: Field> SumOfProductsPoly<F> {
    /// Create a new SumOfProducts instance
    #[inline]
    pub fn new() -> Self {
        Self {
            f_and_g_pairs: vec![],
        }
    }

    /// Get the number of variables in the polynomial
    #[inline]
    pub fn num_vars(&self) -> usize {
        if self.f_and_g_pairs.is_empty() {
            0
        } else {
            self.f_and_g_pairs[0].0.num_vars()
        }
    }

    #[inline]
    pub fn add_pair(&mut self, poly0: MultiLinearPoly<F>, poly1: MultiLinearPoly<F>) {
        assert_eq!(poly0.num_vars(), poly1.num_vars());
        self.f_and_g_pairs.push((poly0, poly1));
    }

    #[inline]
    pub fn evaluate(&self, point: &[F]) -> F {
        self.f_and_g_pairs
            .iter()
            .map(|(f, g)| f.eval_reverse_order(point) * g.eval_reverse_order(point))
            .sum()
    }

    #[inline]
    pub fn sum(&self) -> F {
        self.f_and_g_pairs
            .iter()
            .flat_map(|(f, g)| f.coeffs.iter().zip(g.coeffs.iter()).map(|(&f, &g)| f * g))
            .sum::<F>()
    }
}
