pub const M31_PACK_SIZE: usize = 8;
pub const M31_VECTORIZE_SIZE: usize = 1;

#[derive(Debug, Clone, Copy, Default)]
pub struct M31 {}

impl From<usize> for M31 {
    fn from(x: usize) -> Self {
        M31 {}
    }
}
