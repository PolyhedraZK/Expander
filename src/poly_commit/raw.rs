//! RAW commitment refers to the case where the prover does not commit to the witness at all.
//! The prover will send the whole witnesses to the verifier.

use arith::{FiatShamirConfig, Field, FieldSerde, MultiLinearPoly};

pub struct RawOpening {}

pub struct RawCommitment<F> {
    pub poly_vals: Vec<F>,
}

impl<F: Field + FieldSerde> RawCommitment<F> {
    pub fn size(&self) -> usize {
        self.poly_vals.len() * F::SIZE
    }
    pub fn serialize_into(&self, buffer: &mut [u8]) {
        self.poly_vals
            .iter()
            .enumerate()
            .for_each(|(i, v)| v.serialize_into(&mut buffer[i * F::SIZE..(i + 1) * F::SIZE]));
    }
    pub fn deserialize_from(buffer: &[u8], poly_size: usize) -> Self {
        let mut poly_vals = Vec::new();
        for i in 0..poly_size {
            poly_vals.push(F::deserialize_from(&buffer[i * F::SIZE..(i + 1) * F::SIZE]));
        }
        RawCommitment { poly_vals }
    }
}

impl<F: Field + FieldSerde + FiatShamirConfig> RawCommitment<F> {
    pub fn new(poly_vals: Vec<F>) -> Self {
        RawCommitment { poly_vals }
    }
    pub fn verify(&self, x: &[F::ChallengeField], y: F) -> bool {
        y == MultiLinearPoly::<F>::eval_multilinear(&self.poly_vals, x)
    }
}
