use crate::{eval_multilinear, VectorizedM31, M31};

type FPrimitive = M31;
type F = VectorizedM31;

pub struct RawOpening {}

pub struct RawCommitment {
    pub poly_vals: Vec<F>,
}

impl RawCommitment {
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

impl RawCommitment {
    pub fn new(poly_vals: Vec<F>) -> Self {
        RawCommitment { poly_vals }
    }
    pub fn verify(&self, x: &[FPrimitive], y: F) -> bool {
        y == eval_multilinear(&self.poly_vals, x)
    }
}
