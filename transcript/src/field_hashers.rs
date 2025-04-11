mod mimc;
pub use mimc::MiMC5FiatShamirHasher;

#[cfg(test)]
mod mimc_test;

mod poseidon;
pub use poseidon::{PoseidonFiatShamirHasher, PoseidonStateTrait};
