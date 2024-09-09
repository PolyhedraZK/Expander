#[cfg(target_arch = "aarch64")]
pub(crate) mod neon;
#[cfg(target_arch = "aarch64")]
pub type GF2_128x8 = neon::NeonGF2_128x8;

#[cfg(target_arch = "x86_64")]
cfg_if::cfg_if! {
    if #[cfg(feature = "avx256")] {
        mod avx256;
        pub type GF2_128x8 = avx256::AVX256GF2_128x8;
    } else {
        mod avx;
        pub type GF2_128x8 = avx::AVX512GF2_128x8;
    }
}
