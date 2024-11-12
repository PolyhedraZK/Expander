use std::{arch::aarch64::*, mem::transmute};

use crate::GF2;

use super::GF2x8;

pub(crate) fn pack(base_vec: &[GF2]) -> GF2x8 {
    unsafe {
        let lane_shl: int8x8_t = transmute::<[i8; 8], _>([7, 6, 5, 4, 3, 2, 1, 0]);
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

        let shifted_buffer = vshl_u8(buffer, lane_shl);
        let sum = vaddv_u8(shifted_buffer);

        GF2x8 { v: sum }
    }
}
