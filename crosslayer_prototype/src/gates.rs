use arith::Field;
use gkr_field_config::GKRFieldConfig;
use rand::RngCore;

/// A gate whose inputs are from different layers.
#[derive(Debug, Clone)]
pub struct SimpleGate<C: GKRFieldConfig, const INPUT_NUM: usize> {
    pub i_ids: [usize; INPUT_NUM],
    pub o_id: usize,
    pub coef: C::ChallengeField,
}

pub type SimpleGateMul<C> = SimpleGate<C, 2>;
pub type SimpleGateAdd<C> = SimpleGate<C, 1>;
pub type SimpleGateConst<C> = SimpleGate<C, 0>;

impl<C: GKRFieldConfig, const INPUT_NUM: usize> SimpleGate<C, INPUT_NUM> {
    /// located layer refers to the layer where the output of the gate is.
    /// layer_sizes is the number of nodes in each layer.
    pub fn random_for_testing(mut rng: impl RngCore, output_size: usize, input_size: usize) -> Self {
        let mut i_ids = [0; INPUT_NUM];
        for i in 0..INPUT_NUM {
            i_ids[i] = rng.next_u64() as usize % input_size;
        }

        let o_id = rng.next_u64() as usize % output_size;
        let coef = C::ChallengeField::random_unsafe(rng);
        Self { i_ids, o_id, coef }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CrossLayerRelay<C: GKRFieldConfig> {
    pub o_id: usize,
    pub i_id: usize,
    pub i_layer: usize,
    pub coef: C::ChallengeField,
}

impl<C: GKRFieldConfig> CrossLayerRelay<C> {
    pub fn random_for_testing(mut rng: impl RngCore, output_size: usize, input_size: usize, i_layer: usize) -> Self {
        let o_id = rng.next_u64() as usize % output_size;
        let i_id = rng.next_u64() as usize % input_size;
        let coef = C::ChallengeField::ONE; // temporarily support one only
        Self { o_id, i_id, i_layer, coef }
    }
}

