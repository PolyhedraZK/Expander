use crate::{VectorizedM31, M31};

type FPrimitive = M31;
type F = VectorizedM31;

#[derive(Debug, Clone, Default)]
pub struct Proof {
    pub bytes: Vec<u8>,
}

impl Proof {
    pub fn append_u8_slice(&mut self, buffer: &[u8], size: usize) {
        self.bytes.extend_from_slice(&buffer[..size]);
    }
}
