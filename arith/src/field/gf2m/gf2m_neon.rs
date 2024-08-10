use std::arch::aarch64::*;
#[derive(Clone, Copy)]
pub struct NeonGF2M {
    pub v: [poly64x2_t; 2],
}