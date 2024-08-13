use std::arch::aarch64::uint32x4_t;

use crate::Field;

#[derive(Clone, Copy, Debug)]
pub struct GF2_128x4 {
    v: [uint32x4_t; 4],
}
