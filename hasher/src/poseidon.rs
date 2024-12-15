mod compile_time;
pub use compile_time::{compile_time_alpha, compile_time_gcd};

mod impls;
pub use impls::{PoseidonParams, PoseidonState};

mod m31x16_ext3;
pub use m31x16_ext3::PoseidonM31x16Ext3;

#[cfg(test)]
mod tests;
