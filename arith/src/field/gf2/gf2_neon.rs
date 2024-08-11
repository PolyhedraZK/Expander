use std::arch::aarch64::*;
#[derive(Clone, Copy)]
pub struct NeonGF2 {
    pub v: [poly64x2_t; 2],
}

// We don't need this field
