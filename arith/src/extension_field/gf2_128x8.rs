#[cfg(target_arch = "aarch64")]
pub(crate) mod neon;
#[cfg(target_arch = "aarch64")]
pub type GF2_128x8 = neon::NeonGF2_128x8;

#[cfg(target_arch = "x86_64")]
mod avx256;
#[cfg(target_arch = "x86_64")]
mod avx;
#[cfg(target_arch = "x86_64")]
pub type GF2_128x8_256 = avx256::AVX256GF2_128x8;
#[cfg(feature = "avx256")]
pub type GF2_128x8 = avx256::AVX256GF2_128x8;
#[cfg(all(target_arch = "x86_64", not(feature = "avx256")))]
pub type GF2_128x8 = avx::AVX512GF2_128x8;
