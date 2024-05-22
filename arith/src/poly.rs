use crate::{VectorizedM31, M31};

type FPrimitive = M31;
type F = VectorizedM31;

#[derive(Debug, Clone, Default)]
pub struct MultiLinearPoly {
    pub var_num: usize,
    pub evals: Vec<F>,
}

pub fn eval_multilinear(evals: &[F], x: &[FPrimitive]) -> F {
    assert_eq!(1 << x.len(), evals.len());
    let mut scratch = evals.to_vec();
    let mut cur_eval_size = evals.len() >> 1;
    for r in x.iter() {
        // println!("scratch: {:?}", scratch);
        for i in 0..cur_eval_size {
            scratch[i] = scratch[i * 2] + (scratch[i * 2 + 1] - scratch[i * 2]) * r;
        }
        cur_eval_size >>= 1;
    }
    scratch[0]
}
