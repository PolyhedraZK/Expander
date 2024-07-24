use ark_std::{end_timer, start_timer};

use crate::{SimdField, Field};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Definition for an MLE, with an associated type F.
pub struct MultiLinearPoly<F: Field> {
    /// Number of variables in an MLE
    pub var_num: usize,
    /// MLE Evaluations
    pub evals: Vec<F>,
}

impl<F: Field + SimdField> MultiLinearPoly<F> {
    pub fn eval_multilinear(evals: &[F], x: &[F::Scalar]) -> F {
        let timer = start_timer!(|| format!("eval mle with {} vars", x.len()));
        assert_eq!(1 << x.len(), evals.len());
        let mut scratch = evals.to_vec();
        let mut cur_eval_size = evals.len() >> 1;
        for r in x.iter() {
            log::trace!("scratch: {:?}", scratch);
            for i in 0..cur_eval_size {
                scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]).scale(r);
            }
            cur_eval_size >>= 1;
        }
        end_timer!(timer);
        scratch[0]
    }
}
