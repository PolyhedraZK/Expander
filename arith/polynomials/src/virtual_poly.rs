//! Adapted from the HyperPlonk library, with modifications.
//! https://github.com/EspressoSystems/hyperplonk/blob/main/arithmetic/src/virtual_polynomial.rs

// Copyright (c) 2023 Espresso Systems (espressosys.com)
// This file is part of the HyperPlonk library.

// You should have received a copy of the MIT License
// along with the HyperPlonk library. If not, see <https://mit-license.org/>.

//! This module defines our main mathematical object `VirtualPolynomial`; and
//! various functions associated with it.

use std::{cmp::max, collections::HashMap, marker::PhantomData, ops::Add, sync::Arc};

use arith::Field;
use rand::Rng;
use rand::RngCore;

use crate::MultiLinearPoly;

#[rustfmt::skip]
/// A virtual polynomial is a sum of products of multilinear polynomials;
/// where the multilinear polynomials are stored via their multilinear
/// extensions:  `(coefficient, DenseMultilinearExtension)`
///
/// * Number of products n = `polynomial.products.len()`,
/// * Number of multiplicands of ith product m_i =
///   `polynomial.products[i].1.len()`,
/// * Coefficient of ith product c_i = `polynomial.products[i].0`
///
/// The resulting polynomial is
///
/// $$ \sum_{i=0}^{n} c_i \cdot \prod_{j=0}^{m_i} P_{ij} $$
///
/// Example:
///  f = c0 * f0 * f1 * f2 + c1 * f3 * f4
/// where f0 ... f4 are multilinear polynomials
///
/// - flattened_ml_extensions stores the multilinear extension representation of
///   f0, f1, f2, f3 and f4
/// - products is 
///     \[ 
///         (c0, \[0, 1, 2\]), 
///         (c1, \[3, 4\]) 
///     \]
/// - raw_pointers_lookup_table maps fi to i
///
#[derive(Clone, Debug, Default)]
pub struct VirtualPolynomial<F: Field> {
    /// Aux information about the multilinear polynomial
    pub aux_info: VPAuxInfo<F>,
    /// list of reference to products (as usize) of multilinear extension
    pub products: Vec<(F, Vec<usize>)>,
    /// Stores multilinear extensions in which product multiplicand can refer
    /// to.
    pub flattened_ml_extensions: Vec<Arc<MultiLinearPoly<F>>>,
    /// Pointers to the above poly extensions
    raw_pointers_lookup_table: HashMap<*const MultiLinearPoly<F>, usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// Auxiliary information about the multilinear polynomial
pub struct VPAuxInfo<F: Field> {
    /// max number of multiplicands in each product
    pub max_degree: usize,
    /// number of variables of the polynomial
    pub num_variables: usize,
    /// Associated field
    #[doc(hidden)]
    pub phantom: PhantomData<F>,
}

impl<F: Field> Add for &VirtualPolynomial<F> {
    type Output = VirtualPolynomial<F>;
    fn add(self, other: &VirtualPolynomial<F>) -> Self::Output {
        let mut res = self.clone();
        for products in other.products.iter() {
            let cur: Vec<Arc<MultiLinearPoly<F>>> = products
                .1
                .iter()
                .map(|&x| other.flattened_ml_extensions[x].clone())
                .collect();

            res.add_mle_list(cur, products.0);
        }
        res
    }
}

// TODO: convert this into a trait
impl<F: Field> VirtualPolynomial<F> {
    /// Creates an empty virtual polynomial with `num_variables`.
    pub fn new(num_variables: usize) -> Self {
        VirtualPolynomial {
            aux_info: VPAuxInfo {
                max_degree: 0,
                num_variables,
                phantom: PhantomData,
            },
            products: Vec::new(),
            flattened_ml_extensions: Vec::new(),
            raw_pointers_lookup_table: HashMap::new(),
        }
    }

    /// Creates an new virtual polynomial from a MLE and its coefficient.
    pub fn new_from_mle(mle: &Arc<MultiLinearPoly<F>>, coefficient: F) -> Self {
        let mle_ptr: *const MultiLinearPoly<F> = Arc::as_ptr(mle);
        let mut hm = HashMap::new();
        hm.insert(mle_ptr, 0);

        VirtualPolynomial {
            aux_info: VPAuxInfo {
                // The max degree is the max degree of any individual variable
                max_degree: 1,
                num_variables: mle.get_num_vars(),
                phantom: PhantomData,
            },
            // here `0` points to the first polynomial of `flattened_ml_extensions`
            products: vec![(coefficient, vec![0])],
            flattened_ml_extensions: vec![mle.clone()],
            raw_pointers_lookup_table: hm,
        }
    }

    /// Add a product of list of multilinear extensions to self
    /// Returns an error if the list is empty, or the MLE has a different
    /// `num_vars` from self.
    ///
    /// The MLEs will be multiplied together, and then multiplied by the scalar
    /// `coefficient`.
    pub fn add_mle_list(
        &mut self,
        mle_list: impl IntoIterator<Item = Arc<MultiLinearPoly<F>>>,
        coefficient: F,
    ) {
        let mle_list: Vec<Arc<MultiLinearPoly<F>>> = mle_list.into_iter().collect();
        let mut indexed_product = Vec::with_capacity(mle_list.len());

        if mle_list.is_empty() {
            panic!("input mle_list is empty");
        }

        self.aux_info.max_degree = max(self.aux_info.max_degree, mle_list.len());

        for mle in mle_list {
            if mle.get_num_vars() != self.aux_info.num_variables {
                panic!(
                    "product has a multiplicand with wrong number of variables {} vs {}",
                    mle.get_num_vars(),
                    self.aux_info.num_variables
                );
            }

            let mle_ptr: *const MultiLinearPoly<F> = Arc::as_ptr(&mle);
            if let Some(index) = self.raw_pointers_lookup_table.get(&mle_ptr) {
                indexed_product.push(*index)
            } else {
                let curr_index = self.flattened_ml_extensions.len();
                self.flattened_ml_extensions.push(mle.clone());
                self.raw_pointers_lookup_table.insert(mle_ptr, curr_index);
                indexed_product.push(curr_index);
            }
        }
        self.products.push((coefficient, indexed_product));
    }

    /// Multiple the current VirtualPolynomial by an MLE:
    /// - add the MLE to the MLE list;
    /// - multiple each product by MLE and its coefficient.
    ///
    /// Returns an error if the MLE has a different `num_vars` from self.
    pub fn mul_by_mle(&mut self, mle: Arc<MultiLinearPoly<F>>, coefficient: F) {
        if mle.get_num_vars() != self.aux_info.num_variables {
            panic!(
                "product has a multiplicand with wrong number of variables {} vs {}",
                mle.get_num_vars(),
                self.aux_info.num_variables
            );
        }

        let mle_ptr: *const MultiLinearPoly<F> = Arc::as_ptr(&mle);

        // check if this mle already exists in the virtual polynomial
        let mle_index = match self.raw_pointers_lookup_table.get(&mle_ptr) {
            Some(&p) => p,
            None => {
                self.raw_pointers_lookup_table
                    .insert(mle_ptr, self.flattened_ml_extensions.len());
                self.flattened_ml_extensions.push(mle);
                self.flattened_ml_extensions.len() - 1
            }
        };

        for (prod_coef, indices) in self.products.iter_mut() {
            // - add the MLE to the MLE list;
            // - multiple each product by MLE and its coefficient.
            indices.push(mle_index);
            *prod_coef *= coefficient;
        }

        // increase the max degree by one as the MLE has degree 1.
        self.aux_info.max_degree += 1;
    }

    /// Evaluate the virtual polynomial at point `point`.
    /// Returns an error is point.len() does not match `num_variables`.
    pub fn evaluate(&self, point: &[F]) -> F {
        if self.aux_info.num_variables != point.len() {
            panic!(
                "wrong number of variables {} vs {}",
                self.aux_info.num_variables,
                point.len()
            );
        }

        let evals: Vec<F> = self
            .flattened_ml_extensions
            .iter()
            .map(|x| x.evaluate_jolt(point))
            .collect();

        self.products
            .iter()
            .map(|(c, p)| *c * p.iter().map(|&i| evals[i]).product::<F>())
            .sum()
    }

    /// Sample a random virtual polynomial, return the polynomial and its sum.
    pub fn rand<R: RngCore>(
        nv: usize,
        num_multiplicands_range: (usize, usize),
        num_products: usize,
        rng: &mut R,
    ) -> (Self, F) {
        let mut sum = F::zero();
        let mut poly = VirtualPolynomial::new(nv);
        for _ in 0..num_products {
            let num_multiplicands =
                rng.gen_range(num_multiplicands_range.0..num_multiplicands_range.1);
            let (product, product_sum) = random_mle_list(nv, num_multiplicands, rng);
            let coefficient = F::random_unsafe(&mut *rng);
            poly.add_mle_list(product.into_iter(), coefficient);
            sum += product_sum * coefficient;
        }

        (poly, sum)
    }

    /// Sample a random virtual polynomial that evaluates to zero everywhere
    /// over the boolean hypercube.
    pub fn rand_zero<R: RngCore>(
        nv: usize,
        num_multiplicands_range: (usize, usize),
        num_products: usize,
        rng: &mut R,
    ) -> Self {
        let mut poly = VirtualPolynomial::new(nv);
        for _ in 0..num_products {
            let num_multiplicands =
                rng.gen_range(num_multiplicands_range.0..num_multiplicands_range.1);
            let product = random_zero_mle_list(nv, num_multiplicands, rng);
            let coefficient = F::random_unsafe(&mut *rng);
            poly.add_mle_list(product.into_iter(), coefficient);
        }

        poly
    }

    // // Input poly f(x) and a random vector r, output
    // //      \hat f(x) = \sum_{x_i \in eval_x} f(x_i) eq(x, r)
    // // where
    // //      eq(x,y) = \prod_i=1^num_var (x_i * y_i + (1-x_i)*(1-y_i))
    // //
    // // This function is used in ZeroCheck.
    // pub fn build_f_hat(&self, r: &[F]) -> Self {
    //     if self.aux_info.num_variables != r.len() {
    //         panic!(
    //             "r.len() is different from number of variables {} vs {}",
    //             r.len(),
    //             self.aux_info.num_variables
    //         );
    //     }

    //     let eq_x_r = build_eq_x_r(r);
    //     let mut res = self.clone();
    //     res.mul_by_mle(eq_x_r, F::one());

    //     res
    // }

    // /// Print out the evaluation map for testing. Panic if the num_vars > 5.
    // pub fn print_evals(&self) {
    //     if self.aux_info.num_variables > 5 {
    //         panic!("this function is used for testing only. cannot print more than 5 num_vars")
    //     }
    //     for i in 0..1 << self.aux_info.num_variables {
    //         let point = bit_decompose(i, self.aux_info.num_variables);
    //         let point_fr: Vec<F> = point.iter().map(|&x| F::from(x)).collect();
    //         println!("{} {}", i, self.evaluate(point_fr.as_ref()).unwrap())
    //     }
    //     println!()
    // }
}

// /// Evaluate eq polynomial.
// pub fn eq_eval<F: Field>(x: &[F], y: &[F]) -> F {
//     let mut res = F::one();
//     for (&xi, &yi) in x.iter().zip(y.iter()) {
//         let xi_yi = xi * yi;
//         res *= xi_yi + xi_yi - xi - yi + F::one();
//     }
//     res
// }

// /// This function build the eq(x, r) polynomial for any given r.
// ///
// /// Evaluate
// ///      eq(x,y) = \prod_i=1^num_var (x_i * y_i + (1-x_i)*(1-y_i))
// /// over r, which is
// ///      eq(x,y) = \prod_i=1^num_var (x_i * r_i + (1-x_i)*(1-r_i))
// pub fn build_eq_x_r<F: Field>(r: &[F]) -> Arc<MultiLinearPoly<F>> {
//     let evals = build_eq_x_r_vec(r);
//     let mle = MultiLinearPoly { coeffs: evals };

//     Arc::new(mle)
// }
// /// This function build the eq(x, r) polynomial for any given r, and output the
// /// evaluation of eq(x, r) in its vector form.
// ///
// /// Evaluate
// ///      eq(x,y) = \prod_i=1^num_var (x_i * y_i + (1-x_i)*(1-y_i))
// /// over r, which is
// ///      eq(x,y) = \prod_i=1^num_var (x_i * r_i + (1-x_i)*(1-r_i))
// pub fn build_eq_x_r_vec<F: Field>(r: &[F]) -> Vec<F> {
//     // we build eq(x,r) from its evaluations
//     // we want to evaluate eq(x,r) over x \in {0, 1}^num_vars
//     // for example, with num_vars = 4, x is a binary vector of 4, then
//     //  0 0 0 0 -> (1-r0)   * (1-r1)    * (1-r2)    * (1-r3)
//     //  1 0 0 0 -> r0       * (1-r1)    * (1-r2)    * (1-r3)
//     //  0 1 0 0 -> (1-r0)   * r1        * (1-r2)    * (1-r3)
//     //  1 1 0 0 -> r0       * r1        * (1-r2)    * (1-r3)
//     //  ....
//     //  1 1 1 1 -> r0       * r1        * r2        * r3
//     // we will need 2^num_var evaluations

//     let mut eval = Vec::new();
//     build_eq_x_r_helper(r, &mut eval);
//     eval
// }

// /// A helper function to build eq(x, r) recursively.
// /// This function takes `r.len()` steps, and for each step it requires a maximum
// /// `r.len()-1` multiplications.
// fn build_eq_x_r_helper<F: Field>(r: &[F], buf: &mut Vec<F>) {
//     if r.is_empty() {
//         panic!("r length is 0");
//     } else if r.len() == 1 {
//         // initializing the buffer with [1-r_0, r_0]
//         buf.push(F::one() - r[0]);
//         buf.push(r[0]);
//     } else {
//         build_eq_x_r_helper(&r[1..], buf);

//         // suppose at the previous step we received [b_1, ..., b_k]
//         // for the current step we will need
//         // if x_0 = 0:   (1-r0) * [b_1, ..., b_k]
//         // if x_0 = 1:   r0 * [b_1, ..., b_k]
//         // let mut res = vec![];
//         // for &b_i in buf.iter() {
//         //     let tmp = r[0] * b_i;
//         //     res.push(b_i - tmp);
//         //     res.push(tmp);
//         // }
//         // *buf = res;

//         let mut res = vec![F::zero(); buf.len() << 1];
//         res.iter_mut().enumerate().for_each(|(i, val)| {
//             let bi = buf[i >> 1];
//             let tmp = r[0] * bi;
//             if i & 1 == 0 {
//                 *val = bi - tmp;
//             } else {
//                 *val = tmp;
//             }
//         });
//         *buf = res;
//     }
// }

// /// Decompose an integer into a binary vector in little endian.
// pub fn bit_decompose(input: u64, num_var: usize) -> Vec<bool> {
//     let mut res = Vec::with_capacity(num_var);
//     let mut i = input;
//     for _ in 0..num_var {
//         res.push(i & 1 == 1);
//         i >>= 1;
//     }
//     res
// }

/// Sample a random list of multilinear polynomials.
/// Returns
/// - the list of polynomials,
/// - its sum of polynomial evaluations over the boolean hypercube.
pub fn random_mle_list<F: Field, R: RngCore>(
    nv: usize,
    degree: usize,
    rng: &mut R,
) -> (Vec<Arc<MultiLinearPoly<F>>>, F) {
    let mut multiplicands = Vec::with_capacity(degree);
    for _ in 0..degree {
        multiplicands.push(Vec::with_capacity(1 << nv))
    }
    let mut sum = F::zero();

    for _ in 0..(1 << nv) {
        let mut product = F::one();

        for e in multiplicands.iter_mut() {
            let val = F::random_unsafe(&mut *rng);
            e.push(val);
            product *= val;
        }
        sum += product;
    }

    let list = multiplicands
        .into_iter()
        .map(|x| Arc::new(MultiLinearPoly { coeffs: x }))
        .collect();

    (list, sum)
}

// Build a randomize list of mle-s whose sum is zero.
pub fn random_zero_mle_list<F: Field, R: RngCore>(
    nv: usize,
    degree: usize,
    rng: &mut R,
) -> Vec<Arc<MultiLinearPoly<F>>> {
    let mut multiplicands = Vec::with_capacity(degree);
    for _ in 0..degree {
        multiplicands.push(Vec::with_capacity(1 << nv))
    }
    for _ in 0..(1 << nv) {
        multiplicands[0].push(F::zero());
        for e in multiplicands.iter_mut().skip(1) {
            e.push(F::random_unsafe(&mut *rng));
        }
    }

    let list = multiplicands
        .into_iter()
        .map(|x| Arc::new(MultiLinearPoly { coeffs: x }))
        .collect();

    list
}
