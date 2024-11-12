use std::arch::x86_64::*;
use std::mem::transmute;

use crate::GF2;

use super::GF2x8;

pub(crate) fn pack(base_vec: &[GF2]) -> GF2x8 {
    unsafe {
        let lane_shl: __m128i = transmute::<[u16; 8], _>([7, 6, 5, 4, 3, 2, 1, 0]);
        let zero: __m128i = transmute::<[u64; 2], _>([0, 0]);
        let buffer: __m128i = transmute::<[u16; 8], _>([
            base_vec[0].v as u16,
            base_vec[1].v as u16,
            base_vec[2].v as u16,
            base_vec[3].v as u16,
            base_vec[4].v as u16,
            base_vec[5].v as u16,
            base_vec[6].v as u16,
            base_vec[7].v as u16,
        ]);

        let shifted_buffer = _mm_sllv_epi16(buffer, lane_shl);
        let sum_s = _mm_sad_epu8(shifted_buffer, zero);

        let [sum_0, sum_1] = transmute::<__m128i, [u64; 2]>(sum_s);

        let ret = (sum_0 + sum_1) as u8;
        GF2x8 { v: ret }
    }
}
