/// A M31x16 stores 512 bits of data.
/// With AVX it stores a single __m512i element.
/// With NEON it stores four uint32x4_t elements.
#[cfg(target_arch = "x86_64")]
pub type M31x16 = super::m31_avx::AVXM31;

#[cfg(target_arch = "aarch64")]
pub type M31x16 = super::m31_neon::NeonM31;
