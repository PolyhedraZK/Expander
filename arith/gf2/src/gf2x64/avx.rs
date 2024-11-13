use crate::GF2;

use super::GF2x64;

pub(crate) fn simd_pack_gf2x64(base_vec: &[GF2]) -> GF2x64 {
    assert!(base_vec.len() == GF2x64::PACK_SIZE);
    let mut ret = 0u64;
    for (i, scalar) in base_vec.iter().enumerate() {
        ret |= (scalar.v as u64) << (GF2x64::PACK_SIZE - 1 - i);
    }
    Self { v: ret }
}
