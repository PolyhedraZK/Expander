use arith::Field;

use crate::MultiLinearPoly;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProductOfMLEs<F: Field> {
    pub polynomials: Vec<MultiLinearPoly<F>>,
}

impl<F: Field> ProductOfMLEs<F> {
    /// Create a new Product of MLEs instance
    #[inline]
    pub fn new() -> Self {
        Self {
            polynomials: vec![],
        }
    }

    /// Get the number of variables in the polynomial
    #[inline]
    pub fn num_vars(&self) -> usize {
        if self.polynomials.is_empty() {
            0
        } else {
            self.polynomials[0].get_num_vars()
        }
    }

    #[inline]
    /// Multiplicative degree of the product of MLEs
    pub fn degree(&self) -> usize {
        self.polynomials.len()
    }

    #[inline]
    pub fn from_pair(poly0: MultiLinearPoly<F>, poly1: MultiLinearPoly<F>) -> Self {
        assert_eq!(
            poly0.get_num_vars(),
            poly1.get_num_vars(),
            "Both polynomials must have the same number of variables"
        );

        Self {
            polynomials: vec![poly0, poly1],
        }
    }

    #[inline]
    pub fn from_tuple(
        poly0: MultiLinearPoly<F>,
        poly1: MultiLinearPoly<F>,
        poly2: MultiLinearPoly<F>,
    ) -> Self {
        assert_eq!(
            poly0.get_num_vars(),
            poly1.get_num_vars(),
            "All polynomials must have the same number of variables"
        );
        assert_eq!(
            poly0.get_num_vars(),
            poly2.get_num_vars(),
            "All polynomials must have the same number of variables"
        );

        Self {
            polynomials: vec![poly0, poly1, poly2],
        }
    }

    #[inline]
    pub fn evaluate(&self, point: &[F]) -> F {
        assert_eq!(point.len(), self.num_vars());
        self.polynomials
            .iter()
            .map(|poly| poly.eval_reverse_order(point))
            .product()
    }

    #[inline]
    pub fn sum(&self) -> F {
        match self.degree() {
            2 => self.polynomials[0]
                .coeffs
                .iter()
                .zip(self.polynomials[1].coeffs.iter())
                .map(|(&f, &g)| f * g)
                .sum::<F>(),
            3 => self.polynomials[0]
                .coeffs
                .iter()
                .zip(self.polynomials[1].coeffs.iter())
                .zip(self.polynomials[2].coeffs.iter())
                .map(|((&f, &g), &h)| f * g * h)
                .sum::<F>(),
            _ => panic!("Sum is only defined for degree 2 or 3 polynomials"),
        }
    }

    #[inline]
    pub fn fix_top_variable(&mut self, value: F) {
        for poly in &mut self.polynomials {
            poly.fix_top_variable(value);
        }
    }

    #[inline]
    pub fn extrapolate_at_0_1_2(&self) -> (F, F, F) {
        // evaluate the polynomial at 0, 1 and 2
        // and obtain f(0)g(0) and f(1)g(1) and f(2)g(2)
        assert_eq!(
            self.degree(),
            2,
            "Extrapolation is only defined for degree 2 polynomials"
        );

        let f_coeffs = self.polynomials[0].coeffs.as_slice();
        let g_coeffs = self.polynomials[1].coeffs.as_slice();

        let len = 1 << (self.num_vars() - 1);

        let h_0_local = f_coeffs[..len]
            .iter()
            .zip(g_coeffs[..len].iter())
            .map(|(&f, &g)| f * g)
            .sum::<F>();

        let h_1_local = f_coeffs[len..]
            .iter()
            .zip(g_coeffs[len..].iter())
            .map(|(&f, &g)| f * g)
            .sum::<F>();

        let h_2_local = f_coeffs[..len]
            .iter()
            .zip(f_coeffs[len..].iter())
            .map(|(a, b)| -*a + b.double())
            .zip(
                g_coeffs[..len]
                    .iter()
                    .zip(g_coeffs[len..].iter())
                    .map(|(a, b)| -*a + b.double()),
            )
            .map(|(a, b)| a * b)
            .sum::<F>();

        (h_0_local, h_1_local, h_2_local)
    }
}
