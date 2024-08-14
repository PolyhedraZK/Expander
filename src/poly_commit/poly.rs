use ark_std::{end_timer, start_timer};

use crate::GKRConfig;
use arith::SimdField;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultiLinearPoly {}

impl MultiLinearPoly {
    pub fn eval_circuit_vals_at_challenge<C: GKRConfig>(
        evals: &[C::SimdCircuitField],
        x: &[C::ChallengeField],
        scratch: &mut [C::Field],
    ) -> C::Field {
        let timer = start_timer!(|| format!("eval mle with {} vars", x.len()));
        assert_eq!(1 << x.len(), evals.len());

        let ret = if x.is_empty() {
            C::simd_circuit_field_into_field(&evals[0])
        } else {
            for i in 0..(evals.len() >> 1) {
                scratch[i] = C::field_add_simd_circuit_field(
                    &C::simd_circuit_field_mul_challenge_field(
                        &(evals[i * 2 + 1] - evals[i * 2]),
                        &x[0],
                    ),
                    &evals[i * 2],
                );
            }

            let mut cur_eval_size = evals.len() >> 2;
            for r in x.iter().skip(1) {
                for i in 0..cur_eval_size {
                    scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]).scale(r);
                }
                cur_eval_size >>= 1;
            }
            scratch[0]
        };
        end_timer!(timer);

        ret
    }

    // pub fn eval_multilinear(evals: &[F], x: &[F::Scalar]) -> F {
    //     let timer = start_timer!(|| format!("eval mle with {} vars", x.len()));
    //     assert_eq!(1 << x.len(), evals.len());
    //     let mut scratch = evals.to_vec();
    //     let mut cur_eval_size = evals.len() >> 1;
    //     for r in x.iter() {
    //         log::trace!("scratch: {:?}", scratch);
    //         for i in 0..cur_eval_size {
    //             scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]).scale(r);
    //         }
    //         cur_eval_size >>= 1;
    //     }
    //     end_timer!(timer);
    //     scratch[0]
    // }
}
