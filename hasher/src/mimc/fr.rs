use halo2curves::bn256::Fr;

use crate::FieldHasherState;

impl FieldHasherState for Fr {
    type InputF = Fr;

    type OutputF = Fr;

    const STATE_WIDTH: usize = 1;

    const NAME: &'static str = "MiMC BN254 Fr Field Hasher State";

    fn from_elems(elems: &[Self::InputF]) -> Self {
        assert_eq!(elems.len(), Self::STATE_WIDTH);
        elems[0]
    }

    fn to_elems(&self) -> Vec<Self::InputF> {
        vec![*self]
    }

    fn digest(&self) -> Self::OutputF {
        *self
    }
}
