use gkr_field_config::GKRFieldConfig;

// A direct copy of the witness struct from ecc
#[derive(Debug, Clone)]
pub struct Witness<C: GKRFieldConfig> {
    pub num_witnesses: usize,
    pub num_private_inputs_per_witness: usize,
    pub num_public_inputs_per_witness: usize,
    pub values: Vec<C::CircuitField>,
}
