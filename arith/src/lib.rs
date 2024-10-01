#![cfg_attr(target_arch = "x86_64", feature(stdarch_x86_avx512))]

mod field;
pub use field::*;

mod extension_field;
pub use extension_field::*;

mod simd_field;
pub use simd_field::*;

mod serde;
pub use serde::*;

mod macros;

#[cfg(test)]
mod tests;
