#[cfg(target_arch = "x86_64")]
mod avx;
#[cfg(target_arch = "x86_64")]
pub type GF2x128 = avx::AVXGF2x128;

#[cfg(target_arch = "aarch64")]
mod neon;
#[cfg(target_arch = "aarch64")]
pub type GF2x128 = neon::NeonGF2x128;
