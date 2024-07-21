/// A VectorizedM31 stores 256 bits of data.
/// With AVX it stores a single __m256i element.
/// With NEON it stores two uint32x4_t elements.
#[cfg(target_arch = "x86_64")]
pub type VectorizedM31 = super::m31_avx::AVXM31;

#[cfg(target_arch = "aarch64")]
pub type VectorizedM31 = super::m31_neon::NeonM31;
