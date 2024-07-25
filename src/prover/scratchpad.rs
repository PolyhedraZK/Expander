use arith::{Field, SimdField};

use crate::GKRConfig;

#[derive(Clone, Debug, Default)]
pub struct GkrScratchpad<C: GKRConfig> {
    pub(crate) v_evals: Vec<C::Field>,
    pub(crate) hg_evals: Vec<C::Field>,

    pub(crate) eq_evals_at_rx: Vec<C::ChallengeField>,
    pub(crate) eq_evals_at_rz0: Vec<C::ChallengeField>,
    pub(crate) eq_evals_at_rz1: Vec<C::ChallengeField>,
    pub(crate) eq_evals_first_half: Vec<C::ChallengeField>,
    pub(crate) eq_evals_second_half: Vec<C::ChallengeField>,

    pub(crate) gate_exists: Vec<bool>,
}

impl<C: GKRConfig> GkrScratchpad<C> {
    pub(crate) fn new(max_num_input_var: usize, max_num_output_var: usize) -> Self {
        let max_input_num = 1 << max_num_input_var;
        let max_output_num = 1 << max_num_output_var;
        GkrScratchpad {
            v_evals: vec![C::Field::default(); max_input_num],
            hg_evals: vec![C::Field::default(); max_input_num],

            eq_evals_at_rx: vec![C::ChallengeField::default(); max_input_num],
            eq_evals_at_rz0: vec![C::ChallengeField::default(); max_output_num],
            eq_evals_at_rz1: vec![C::ChallengeField::default(); max_output_num],
            eq_evals_first_half: vec![C::ChallengeField::default(); max_output_num],
            eq_evals_second_half: vec![C::ChallengeField::default(); max_output_num],

            gate_exists: vec![false; max_input_num],
        }
    }
}
