use arith::Field;

use crate::{MultiLinearPoly, MultilinearExtension, ProductOfMLEs};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// A special form of a multi-linear polynomial: f = f0*g0 + f1*g1 + ...
/// where f0, f1, ...  and g0, g1, ... are multi-linear polynomials
/// The sumcheck over this polynomial has a degree of 2
pub struct SumOfProductsPoly<F: Field> {
    /// The list of multi-linear polynomials to be summed
    pub monomials: Vec<ProductOfMLEs<F>>,
}

impl<F: Field> SumOfProductsPoly<F> {
    /// Create a new SumOfProducts instance
    #[inline]
    pub fn new() -> Self {
        Self { monomials: vec![] }
    }

    /// Get the number of variables in the polynomial
    #[inline]
    pub fn num_vars(&self) -> usize {
        if self.monomials.is_empty() {
            0
        } else {
            self.monomials[0].num_vars()
        }
    }

    #[inline]
    pub fn add_pair(&mut self, poly0: MultiLinearPoly<F>, poly1: MultiLinearPoly<F>) {
        if !self.monomials.is_empty() {
            // Ensure new pair's num_vars matches existing pairs.
            assert_eq!(
                self.num_vars(),
                poly0.num_vars(),
                "All polynomial pairs must have the same number of variables"
            );
        }
        self.monomials.push(ProductOfMLEs::from_pair(poly0, poly1));
    }

    #[inline]
    pub fn evaluate(&self, point: &[F]) -> F {
        self.monomials.iter().map(|m| m.evaluate(point)).sum()
    }

    #[inline]
    pub fn sum(&self) -> F {
        self.monomials.iter().map(|m| m.sum()).sum()
    }

    #[inline]
    pub fn fix_top_variable(&mut self, value: F) {
        for monomial in &mut self.monomials {
            monomial.fix_top_variable(value);
        }
    }

    #[inline]
    pub fn extrapolate_at_0_1_2(&self) -> (F, F, F) {
        let mut sum0 = F::zero();
        let mut sum1 = F::zero();
        let mut sum2 = F::zero();

        for monomial in &self.monomials {
            let (s0, s1, s2) = monomial.extrapolate_at_0_1_2();
            sum0 += s0;
            sum1 += s1;
            sum2 += s2;
        }

        (sum0, sum1, sum2)
    }
}
