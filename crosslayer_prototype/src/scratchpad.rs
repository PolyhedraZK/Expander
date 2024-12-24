//! Scratch pad for prover and verifier to store intermediate values during the sumcheck protocol.

use std::cmp::max;

use arith::Field;
use gkr_field_config::GKRFieldConfig;

#[derive(Clone, Debug, Default)]
pub struct CrossLayerProverScratchPad<C: GKRFieldConfig> {
    pub v_evals: Vec<C::Field>,
    pub hg_evals: Vec<C::Field>,

    pub cross_layer_sizes: Vec<usize>,
    pub cross_layer_circuit_vals: Vec<Vec<C::SimdCircuitField>>,
    pub cross_layer_evals: Vec<Vec<C::Field>>,
    pub cross_layer_hg_evals: Vec<Vec<C::Field>>,
    pub cross_layer_completed_values: Vec<C::Field>,
    pub eq_evals_at_r_simd_at_layer: Vec<Vec<C::ChallengeField>>,

    pub simd_var_v_evals: Vec<C::ChallengeField>,
    pub simd_var_hg_evals: Vec<C::ChallengeField>,

    pub eq_evals_at_rx: Vec<C::ChallengeField>,
    pub eq_evals_at_rz0: Vec<C::ChallengeField>,
    pub eq_evals_at_rz1: Vec<C::ChallengeField>,
    pub eq_evals_at_r_simd: Vec<C::ChallengeField>,

    pub eq_evals_first_half: Vec<C::ChallengeField>,
    pub eq_evals_second_half: Vec<C::ChallengeField>,

    pub phase2_coef: C::ChallengeField,
}

impl<C: GKRFieldConfig> CrossLayerProverScratchPad<C> {
    pub fn new(
        n_layers: usize,
        max_num_input_var: usize,
        max_num_output_var: usize,
        mpi_world_size: usize,
    ) -> Self {
        let max_input_num = 1 << max_num_input_var;
        let max_output_num = 1 << max_num_output_var;
        CrossLayerProverScratchPad {
            v_evals: vec![C::Field::default(); max_input_num],
            hg_evals: vec![C::Field::default(); max_input_num],

            simd_var_v_evals: vec![C::ChallengeField::default(); C::get_field_pack_size()],
            simd_var_hg_evals: vec![C::ChallengeField::default(); C::get_field_pack_size()],

            // To be initialized in the sumcheck protocol
            cross_layer_sizes: vec![0; n_layers],
            cross_layer_circuit_vals: vec![vec![]; n_layers],
            cross_layer_evals: vec![vec![]; n_layers],
            cross_layer_hg_evals: vec![vec![]; n_layers],
            cross_layer_completed_values: vec![C::Field::ONE; n_layers],
            eq_evals_at_r_simd_at_layer: vec![
                vec![C::ChallengeField::ONE; C::get_field_pack_size()];
                n_layers
            ],

            eq_evals_at_rx: vec![C::ChallengeField::default(); max_input_num],
            eq_evals_at_rz0: vec![C::ChallengeField::default(); max_output_num],
            eq_evals_at_rz1: vec![C::ChallengeField::default(); max_output_num],
            eq_evals_at_r_simd: vec![C::ChallengeField::default(); C::get_field_pack_size()],
            eq_evals_first_half: vec![
                C::ChallengeField::default();
                max(
                    max(max_output_num, C::get_field_pack_size()),
                    mpi_world_size
                )
            ],
            eq_evals_second_half: vec![
                C::ChallengeField::default();
                max(
                    max(max_output_num, C::get_field_pack_size()),
                    mpi_world_size
                )
            ],
            phase2_coef: C::ChallengeField::ZERO,
        }
    }
}
