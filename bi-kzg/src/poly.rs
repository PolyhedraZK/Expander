pub mod coef_form;
pub mod lagrange_form;

pub mod utils;
pub use utils::*;

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
