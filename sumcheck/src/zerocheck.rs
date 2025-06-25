//! credit: https://github.com/EspressoSystems/hyperplonk/blob/main/subroutines/src/poly_iop/zero_check/mod.rs

use arith::Field;

/// A zero check IOP subclaim for `f(x)` consists of the following:
///   - the initial challenge vector r which is used to build eq(x, r) in SumCheck
///   - the random vector `v` to be evaluated
///   - the claimed evaluation of `f(v)`
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ZeroCheckSubClaim<F: Field> {
    // the evaluation point
    pub point: Vec<F>,
    /// the expected evaluation
    pub expected_evaluation: F,
    // the initial challenge r which is used to build eq(x, r)
    pub init_challenge: Vec<F>,
}

pub struct ZeroCheck<F: Field> {
    phantom: std::marker::PhantomData<F>,
}

impl<F: Field> ZeroCheck<F> {}
