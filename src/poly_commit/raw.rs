//! RAW commitment refers to the case where the prover does not commit to the witness at all.
//! The prover will send the whole witnesses to the verifier.

use std::io::{Read, Write};

use arith::{FiatShamirConfig, Field, FieldSerde, MultiLinearPoly};

pub struct RawOpening {}

pub struct RawCommitment<F> {
    pub poly_vals: Vec<F>,
}

impl<F: Field + FieldSerde> RawCommitment<F> {
    #[inline]
    pub fn size(&self) -> usize {
        self.poly_vals.len() * F::SIZE
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
            .map(|_| F::deserialize_from(&mut reader))
            .collect();

        RawCommitment { poly_vals }
    }
}

impl<F: Field + FieldSerde + FiatShamirConfig> RawCommitment<F> {
    #[inline]
    pub fn new(poly_vals: Vec<F>) -> Self {
        RawCommitment { poly_vals }
    }

    #[inline]
    pub fn verify(&self, x: &[F::ChallengeField], y: F) -> bool {
        y == MultiLinearPoly::<F>::eval_multilinear(&self.poly_vals, x)
    }
}
