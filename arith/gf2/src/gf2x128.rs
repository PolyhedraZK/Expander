use std::mem::transmute;

use arith::{Field, SimdField};

use crate::{GF2x64, GF2};

#[cfg(target_arch = "x86_64")]
mod avx;
#[cfg(target_arch = "x86_64")]
pub type GF2x128 = avx::AVXGF2x128;

#[cfg(target_arch = "aarch64")]
mod neon;
#[cfg(target_arch = "aarch64")]
pub type GF2x128 = neon::NeonGF2x128;

impl SimdField for GF2x128 {
    type Scalar = GF2;

    const PACK_SIZE: usize = 128;

    #[inline(always)]
    fn scale(&self, challenge: &Self::Scalar) -> Self {
        if challenge.v == 0 {
            <Self as Field>::ZERO
        } else {
            *self
        }
    }

    #[inline(always)]
    fn pack(base_vec: &[Self::Scalar]) -> Self {
        assert_eq!(base_vec.len(), Self::PACK_SIZE);
        let mut packed_to_gf2x64 = [GF2x64::ZERO; Self::PACK_SIZE / GF2x64::PACK_SIZE];
        packed_to_gf2x64
            .iter_mut()
            .rev()
            .zip(base_vec.chunks(GF2x64::PACK_SIZE))
            .for_each(|(gf2x64, pack)| *gf2x64 = GF2x64::pack(pack));

        unsafe { transmute(packed_to_gf2x64) }
    }

    #[inline(always)]
    fn unpack(&self) -> Vec<Self::Scalar> {
        let packed_to_gf2x64: [GF2x64; Self::PACK_SIZE / GF2x64::PACK_SIZE] =
            unsafe { transmute(*self) };

        packed_to_gf2x64
            .iter()
            .rev()
            .flat_map(|packed| packed.unpack())
            .collect()
    }
}
