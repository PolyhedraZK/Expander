use arith::{Field, FieldSerde, VectorizedM31};

type F = VectorizedM31;

/// Proof. In the serialized mode.
#[derive(Debug, Clone, Default)]
pub struct Proof {
    idx: usize,
    // ZZ: shall we use Vec<[u8; F::SIZE]> so we can remove idx field?
    pub bytes: Vec<u8>,
}

impl Proof {
    // ZZ: may be all the functions here can be pub(crate)?
    #[inline(always)]
    pub fn append_u8_slice(&mut self, buffer: &[u8], size: usize) {
        self.bytes.extend_from_slice(&buffer[..size]);
    }

    #[inline(always)]
    pub fn step(&mut self, size: usize) {
        self.idx += size;
    }

    #[inline(always)]
    pub fn get_next_and_step(&mut self) -> F {
        let ret = F::deserialize_from(&self.bytes[self.idx..]);
        self.step(F::SIZE);
        ret
    }
}
