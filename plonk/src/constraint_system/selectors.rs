use arith::{Field, UnivariateLagrangePolynomial, UnivariatePolynomial};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Selector<F> {
    // a selector is a vector of field elements which are the FFT values of the selector polynomial
    pub(crate) q: Vec<F>,
}

impl<F: Field> Selector<F> {
    /// get number of variables
    pub(crate) fn get_nv(&self) -> usize {
        self.q.len()
    }

    /// add a new element to the selector
    pub(crate) fn push(&mut self, value: F) {
        self.q.push(value);
    }
}

impl<F: Field> From<&Selector<F>> for UnivariatePolynomial<F> {
    fn from(selector: &Selector<F>) -> Self {
        // let lagrange_poly = UnivariateLagrangePolynomial {
        //     coefficients: selector.q.clone(),
        // };
        // lagrange_poly.into()

        todo!()
    }
}
