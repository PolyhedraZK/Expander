#![cfg_attr(target_arch = "x86_64", feature(stdarch_x86_avx512))]

mod m31x16;
pub use m31x16::M31x16;

#[cfg(target_arch = "x86_64")]
pub(crate) mod m31_avx;
#[cfg(target_arch = "x86_64")]
pub(crate) mod m31_avx256;

#[cfg(target_arch = "x86_64")]
pub type M31x16_256 = m31_avx256::AVXM31;

#[cfg(target_arch = "aarch64")]
pub mod m31_neon;

mod m31;
pub use m31::M31;

mod m31_ext;
pub use m31_ext::M31Ext3;

mod m31_ext3x16;
pub use m31_ext3x16::M31Ext3x16;
