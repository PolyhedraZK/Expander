#[cfg(target_arch = "x86_64")]
pub mod m31_avx;
#[cfg(target_arch = "x86_64")]
pub use m31_avx::*;

#[cfg(target_arch = "arm")]
pub mod m31_neon;
#[cfg(target_arch = "arm")]
pub use m31_neon::*;
