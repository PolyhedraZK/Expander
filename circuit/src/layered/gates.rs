use gkr_field_config::GKRFieldConfig;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum CoefType {
    #[default]
    Constant,
    Random,
    PublicInput(usize),
}

#[derive(Clone, Debug)]
pub struct Gate<C: GKRFieldConfig, const INPUT_NUM: usize> {
    pub i_ids: [usize; INPUT_NUM],
    pub o_id: usize,
    pub coef_type: CoefType,
    pub coef: C::CircuitField,
    pub gate_type: usize,
}

pub type GateMul<C> = Gate<C, 2>;
pub type GateAdd<C> = Gate<C, 1>;
pub type GateUni<C> = Gate<C, 1>;
pub type GateConst<C> = Gate<C, 0>;

// If I Simply add derive(Copy) to the Gate struct, the compiler does not seem to recognize it
// for the type aliases. Explicitly state it here.
impl<C: GKRFieldConfig, const INPUT_NUM: usize> Copy for Gate<C, INPUT_NUM> {}
