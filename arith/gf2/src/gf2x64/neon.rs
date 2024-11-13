use std::{arch::aarch64::*, mem::transmute};

use crate::GF2;

use super::GF2x64;

const LANE_SHL: int8x8_t = unsafe { transmute::<[i8; 8], _>([7, 6, 5, 4, 3, 2, 1, 0]) };

fn simd_pack_gf2x8(base_vec: &[GF2]) -> u8 {
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
        vaddv_u8(shifted_buffer)
    }
}

pub(crate) fn simd_pack_gf2x64(base_vec: &[GF2]) -> GF2x64 {
    let b0 = simd_pack_gf2x8(&base_vec[0..8]) as u64;
    let b1 = simd_pack_gf2x8(&base_vec[8..16]) as u64;
    let b2 = simd_pack_gf2x8(&base_vec[16..24]) as u64;
    let b3 = simd_pack_gf2x8(&base_vec[24..32]) as u64;
    let b4 = simd_pack_gf2x8(&base_vec[32..40]) as u64;
    let b5 = simd_pack_gf2x8(&base_vec[40..48]) as u64;
    let b6 = simd_pack_gf2x8(&base_vec[48..56]) as u64;
    let b7 = simd_pack_gf2x8(&base_vec[56..64]) as u64;

    GF2x64 {
        v: (b0 << 56)
            | (b1 << 48)
            | (b2 << 40)
            | (b3 << 32)
            | (b4 << 24)
            | (b5 << 16)
            | (b6 << 8)
            | b7,
    }
}
