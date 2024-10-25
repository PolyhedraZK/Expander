mod bi_fft;
pub use bi_fft::bi_fft_in_place;

mod coeff;

mod lagrange;
pub use lagrange::*;

use arith::FFTField;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BivariatePolynomial<F> {
    pub coefficients: Vec<F>,
    pub degree_0: usize,
    pub degree_1: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BivariateLagrangePolynomial<F> {
    pub coefficients: Vec<F>,
    pub degree_0: usize,
    pub degree_1: usize,
}

impl<F: FFTField> From<BivariatePolynomial<F>> for BivariateLagrangePolynomial<F> {
    #[inline]
    fn from(poly: BivariatePolynomial<F>) -> Self {
        Self::from(&poly)
    }
}

impl<F: FFTField> From<&BivariatePolynomial<F>> for BivariateLagrangePolynomial<F> {
    #[inline]
    fn from(poly: &BivariatePolynomial<F>) -> Self {
        let coeffs = poly.interpolate();
        BivariateLagrangePolynomial::new(coeffs, poly.degree_0, poly.degree_1)
    }
}
