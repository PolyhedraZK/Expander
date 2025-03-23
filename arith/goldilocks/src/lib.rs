mod goldilocks;
pub use goldilocks::{Goldilocks, EPSILON, GOLDILOCKS_MOD};

mod goldilocks_ext;
pub use goldilocks_ext::GoldilocksExt2;

mod util;

#[cfg(test)]
mod tests;
