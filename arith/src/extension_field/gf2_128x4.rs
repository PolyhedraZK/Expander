#[cfg(target_arch = "aarch64")]
pub(crate) mod neon;
#[cfg(target_arch = "aarch64")]
pub use neon::GF2_128x4;

#[cfg(target_arch = "x86_64")]
mod avx;
#[cfg(target_arch = "x86_64")]
pub use avx::GF2_128x4;
