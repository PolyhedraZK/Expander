use crate::VectorizedM31;

type F = VectorizedM31;

#[derive(Debug, Clone, Default)]
pub struct Proof {
    idx: usize,
    pub bytes: Vec<u8>,
}

impl Proof {
    pub fn append_u8_slice(&mut self, buffer: &[u8], size: usize) {
        self.bytes.extend_from_slice(&buffer[..size]);
    }
    pub fn step(&mut self, size: usize) {
        self.idx += size;
    }
    pub fn get_next_and_step(&mut self) -> F {
        let ret = F::deserialize_from(&self.bytes[self.idx..]);
        self.step(F::SIZE);
        ret
    }
}
