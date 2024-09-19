// A M31x16 stores 512 bits of data.
// With AVX it stores a single __m512i element.
// With NEON it stores four uint32x4_t elements.
#[cfg(target_arch = "x86_64")]
cfg_if::cfg_if! {
    if #[cfg(feature = "avx256")] {
        mod m31_avx256;
        pub type M31x16 = m31_avx256::AVXM31;
    } else {
        mod m31_avx;
        pub type M31x16 = m31_avx::AVXM31;
    }
}

#[cfg(target_arch = "aarch64")]
mod m31_neon;
#[cfg(target_arch = "aarch64")]
pub type M31x16 = super::m31_neon::NeonM31;
