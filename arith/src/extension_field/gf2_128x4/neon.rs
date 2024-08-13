use std::arch::aarch64::uint32x4_t;

#[derive(Clone, Copy, Debug)]
pub struct NeonGF2_128x4 {
    v: [uint32x4_t; 4],
}