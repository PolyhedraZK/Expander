use crate::{VectorizedM31, M31};

type FPrimitive = M31;
type F = VectorizedM31;

#[derive(Debug, Clone, Default)]
pub struct MultiLinearPoly {
    pub var_num: usize,
    pub evals: Vec<F>,
}
