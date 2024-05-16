use crate::M31;

type F = M31;

#[derive(Debug, Clone, Default)]
pub struct MultiLinearPoly {
    pub var_num: usize,
    pub evals: Vec<F>,
}
