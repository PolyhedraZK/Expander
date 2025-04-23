#![cfg_attr(target_arch = "x86_64", feature(stdarch_x86_avx512))]

#[cfg(feature = "proving")]
pub mod prover;
#[cfg(feature = "proving")]
pub use prover::*;

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
