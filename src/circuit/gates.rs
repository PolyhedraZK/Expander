#[derive(Debug, Clone)]
pub struct Gate<C: GKRConfig, const INPUT_NUM: usize> {
    pub i_ids: [usize; INPUT_NUM],
    pub o_id: usize,
    pub coef: C::CircuitField,
    pub is_random: bool,
    pub gate_type: usize,
}

pub type GateMul<C> = Gate<C, 2>;
pub type GateAdd<C> = Gate<C, 1>;
pub type GateUni<C> = Gate<C, 1>;
pub type GateConst<C> = Gate<C, 0>;
