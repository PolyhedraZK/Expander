use crate::{VectorizedM31, M31};

type FPrimitive = M31;
type F = VectorizedM31;

#[derive(Clone, Debug)]
pub struct GkrScratchpad {
    pub v_evals: Vec<F>,
    pub hg_evals: Vec<F>,

    pub eq_evals_at_rx: Vec<FPrimitive>,
    pub eq_evals_at_rz0: Vec<FPrimitive>,
    pub eq_evals_at_rz1: Vec<FPrimitive>,
    pub eq_evals_first_half: Vec<FPrimitive>,
    pub eq_evals_second_half: Vec<FPrimitive>,

    pub gate_exists: Vec<bool>,
}

impl GkrScratchpad {
    pub fn new(max_num_input_var: usize, max_num_output_var: usize) -> Self {
        let max_input_num = 1 << max_num_input_var;
        let max_output_num = 1 << max_num_output_var;
        GkrScratchpad {
            v_evals: vec![F::default(); max_input_num],
            hg_evals: vec![F::default(); max_input_num],

            eq_evals_at_rx: vec![FPrimitive::default(); max_input_num],
            eq_evals_at_rz0: vec![FPrimitive::default(); max_output_num],
            eq_evals_at_rz1: vec![FPrimitive::default(); max_output_num],
            eq_evals_first_half: vec![FPrimitive::default(); max_output_num],
            eq_evals_second_half: vec![FPrimitive::default(); max_output_num],

            gate_exists: vec![false; max_input_num],
        }
    }
}
