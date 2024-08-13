/// A Simdgf2 stores 512 bits of data.
/// With AVX it stores a single __m512i element.
/// With NEON it stores four uint32x4_t elements.
#[cfg(target_arch = "x86_64")]
pub type SimdGF2_128 = super::avx_gf2_128::AVX512GF2_128;

#[cfg(target_arch = "aarch64")]
pub type SimdGF2_128 = super::neon_gf2_128::NeonGF2_128;
