use std::mem::transmute;

use arith::{Field, SimdField};

use crate::{GF2x64, GF2};

#[cfg(target_arch = "aarch64")]
mod neon;
#[cfg(target_arch = "aarch64")]
pub type GF2x512 = neon::NeonGF2x512;

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
mod avx512;
#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub type GF2x512 = avx512::AVX512GF2x512;

// Fallback, use avx2
#[cfg(all(target_arch = "x86_64", not(target_feature = "avx512f")))]
mod avx256;
#[cfg(all(target_arch = "x86_64", not(target_feature = "avx512f")))]
pub type GF2x512 = avx256::AVX256GF2x512;

impl SimdField for GF2x512 {
    type Scalar = GF2;

    const PACK_SIZE: usize = 512;

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
            .flat_map(|packed| packed.unpack())
            .collect()
    }
}
