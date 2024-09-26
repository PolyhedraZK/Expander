// A M31x16 stores 512 bits of data.
// With AVX it stores a single __m512i element.
// With NEON it stores four uint32x4_t elements.

#[cfg(target_arch = "aarch64")]
pub type M31x16 = super::m31_neon::NeonM31;

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub type M31x16 = super::m31_avx512::AVXM31;

#[cfg(all(target_arch = "x86_64", not(target_feature = "avx512f")))]
pub type M31x16 = super::m31_avx256::AVXM31;
