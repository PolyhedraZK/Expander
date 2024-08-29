// use arith::{Field, UnivariateLagrangePolynomial, UnivariatePolynomial};

// #[derive(Clone, Debug, PartialEq, Eq, Default)]
// pub struct Witness {
//     // a selector is a vector of field elements which are the FFT values of the witness polynomial
//     pub(crate) q: Vec<Variable>,
// }

// impl Witness {
//     /// get number of variables
//     #[inline]
//     pub(crate) fn get_nv(&self) -> usize {
//         self.q.len()
//     }

//     /// add a new element to the Witness
//     #[inline]
//     pub(crate) fn add(&mut self, value: &Variable) {
//         self.q.push(*value);
//     }
// }

// // impl From<&Witness<F>> for UnivariatePolynomial<F> {
// //     #[inline]
// //     fn from(selector: &Witness<F>) -> Self {
// //         let lagrange_poly = UnivariateLagrangePolynomial {
// //             coefficients: selector.q.clone(),
// //         };
// //         lagrange_poly.into()
// //     }
// // }
