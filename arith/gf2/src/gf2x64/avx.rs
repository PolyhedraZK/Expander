use std::arch::x86_64::*;
use std::mem::transmute;

use crate::GF2;

use super::GF2x64;

const LANE_SHL: __m256i =
    unsafe { transmute::<[u16; 16], _>([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]) };

fn simd_pack_gf2x16(base_vec: &[GF2]) -> u16 {
    unsafe {
        let buffer: __m256i = transmute::<[u16; 16], _>([
            base_vec[0].v as u16,
            base_vec[1].v as u16,
            base_vec[2].v as u16,
            base_vec[3].v as u16,
            base_vec[4].v as u16,
            base_vec[5].v as u16,
            base_vec[6].v as u16,
            base_vec[7].v as u16,
            base_vec[8].v as u16,
            base_vec[9].v as u16,
            base_vec[10].v as u16,
            base_vec[11].v as u16,
            base_vec[12].v as u16,
            base_vec[13].v as u16,
            base_vec[14].v as u16,
            base_vec[15].v as u16,
        ]);

        let shifted_buffer = _mm256_sllv_epi16(buffer, LANE_SHL);
        let [low_lanes, high_lanes] = transmute::<__m256i, [__m128i; 2]>(shifted_buffer);

        let lane_sums_8x16 = _mm_hadd_epi16(low_lanes, high_lanes);
        let lane_sums_4x16 = _mm_hadd_epi16(lane_sums_8x16, lane_sums_8x16);
        let lane_sums_2x16 = _mm_hadd_epi16(lane_sums_4x16, lane_sums_4x16);
        let lane_sums_1x16 = _mm_hadd_epi16(lane_sums_2x16, lane_sums_2x16);

        _mm_extract_epi16::<0>(lane_sums_1x16) as u16
    }
}

pub(crate) fn simd_pack_gf2x64(base_vec: &[GF2]) -> GF2x64 {
    let b0: u64 = simd_pack_gf2x16(&base_vec[0..16]) as u64;
    let b1: u64 = simd_pack_gf2x16(&base_vec[16..32]) as u64;
    let b2: u64 = simd_pack_gf2x16(&base_vec[32..48]) as u64;
    let b3: u64 = simd_pack_gf2x16(&base_vec[48..64]) as u64;

    GF2x64 {
        v: (b0 << 48) | (b1 << 32) | (b2 << 16) | b3,
    }
}
