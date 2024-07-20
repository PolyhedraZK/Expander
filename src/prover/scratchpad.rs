use arith::Field;

#[derive(Clone, Debug)]
pub struct GkrScratchpad<F: Field> {
    pub(crate) v_evals: Vec<F>,
    pub(crate) hg_evals: Vec<F>,

    pub(crate) eq_evals_at_rx: Vec<F>,
    pub(crate) eq_evals_at_rz0: Vec<F>,
    pub(crate) eq_evals_at_rz1: Vec<F>,
    pub(crate) eq_evals_first_half: Vec<F>,
    pub(crate) eq_evals_second_half: Vec<F>,

    pub(crate) gate_exists: Vec<bool>,
}

impl<F: Field> GkrScratchpad<F> {
    pub(crate) fn new(max_num_input_var: usize, max_num_output_var: usize) -> Self {
        let max_input_num = 1 << max_num_input_var;
        let max_output_num = 1 << max_num_output_var;
        GkrScratchpad {
            v_evals: vec![F::default(); max_input_num],
            hg_evals: vec![F::default(); max_input_num],

            eq_evals_at_rx: vec![F::default(); max_input_num],
            eq_evals_at_rz0: vec![F::default(); max_output_num],
            eq_evals_at_rz1: vec![F::default(); max_output_num],
            eq_evals_first_half: vec![F::default(); max_output_num],
            eq_evals_second_half: vec![F::default(); max_output_num],

            gate_exists: vec![false; max_input_num],
        }
    }
}
