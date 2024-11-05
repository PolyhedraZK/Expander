mod fft;
pub use fft::*;

mod coeff;
pub use coeff::*;

mod lagrange;
// pub use lagrange::*;

use arith::FFTField;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnivariatePolynomial<F> {
    pub coefficients: Vec<F>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnivariateLagrangePolynomial<F> {
    pub coefficients: Vec<F>,
}

impl<F: FFTField> From<UnivariatePolynomial<F>> for UnivariateLagrangePolynomial<F> {
    #[inline]
    fn from(poly: UnivariatePolynomial<F>) -> Self {
        Self::from(&poly)
    }
}

impl<F: FFTField> From<&UnivariatePolynomial<F>> for UnivariateLagrangePolynomial<F> {
    #[inline]
    fn from(poly: &UnivariatePolynomial<F>) -> Self {
        let coeffs = poly.interpolate();
        UnivariateLagrangePolynomial::new(coeffs)
    }
}
