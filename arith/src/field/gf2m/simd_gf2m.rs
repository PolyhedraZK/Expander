/// A Simdgf2m stores 512 bits of data.
/// With AVX it stores a single __m512i element.
/// With NEON it stores four uint32x4_t elements.
#[cfg(target_arch = "x86_64")]
pub type SimdGF2M = super::gf2m_avx512::AVX512GF2M;

#[cfg(target_arch = "aarch64")]
pub type SimdGF2M = super::gf2m_neon::NeonGF2M;
