//! RAW commitment refers to the case where the prover does not commit to the witness at all.
//! The prover will send the whole witnesses to the verifier.

use std::io::{Read, Write};

use arith::{Field, FieldSerde};

use crate::{GKRConfig, MultiLinearPoly};

pub struct RawOpening {}

pub struct RawCommitment<C: GKRConfig> {
    pub poly_vals: Vec<C::SimdCircuitField>,
}

impl<C: GKRConfig> RawCommitment<C> {
    #[inline]
    pub fn size(&self) -> usize {
        self.poly_vals.len() * C::SimdCircuitField::SIZE
    }

    #[inline]
    pub fn serialize_into<W: Write>(&self, mut writer: W) {
        self.poly_vals
            .iter()
            .for_each(|v| v.serialize_into(&mut writer));
    }

    #[inline]
    pub fn deserialize_from<R: Read>(mut reader: R, poly_size: usize) -> Self {
        let poly_vals = (0..poly_size)
            .map(|_| C::SimdCircuitField::deserialize_from(&mut reader))
            .collect();

        RawCommitment { poly_vals }
    }
}

impl<C: GKRConfig> RawCommitment<C> {
    #[inline]
    pub fn new(poly_vals: &[C::SimdCircuitField]) -> Self {
        RawCommitment {
            poly_vals: poly_vals.to_owned(),
        }
    }

    #[inline]
    pub fn verify(&self, x: &[C::ChallengeField], y: C::Field) -> bool {
        let mut scratch = vec![C::Field::default(); self.poly_vals.len()];
        y == MultiLinearPoly::eval_circuit_vals_at_challenge::<C>(&self.poly_vals, x, &mut scratch)
    }
}
