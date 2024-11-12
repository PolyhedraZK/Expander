use std::{arch::aarch64::*, mem::transmute};

use crate::GF2;

use super::GF2x8;

const LANE_SHL: int8x8_t = unsafe { transmute::<[i8; 8], _>([7, 6, 5, 4, 3, 2, 1, 0]) };

pub(crate) fn simd_pack_gf2x8(base_vec: &[GF2]) -> GF2x8 {
    unsafe {
        let buffer: uint8x8_t = transmute([
            base_vec[0].v,
            base_vec[1].v,
            base_vec[2].v,
            base_vec[3].v,
            base_vec[4].v,
            base_vec[5].v,
            base_vec[6].v,
            base_vec[7].v,
        ]);

        let shifted_buffer = vshl_u8(buffer, LANE_SHL);
        let sum = vaddv_u8(shifted_buffer);

        GF2x8 { v: sum }
    }
}
