#![cfg_attr(target_arch = "x86_64", feature(stdarch_x86_avx512))]

pub mod linear_gkr;
pub use linear_gkr::*;

pub mod verifier;
pub use verifier::*;

pub mod utils;

pub mod executor;

pub mod gkr_configs;
pub use gkr_configs::*;

#[cfg(test)]
mod tests;

#[cfg(feature = "grinding")]
const GRINDING_BITS: usize = 10;
