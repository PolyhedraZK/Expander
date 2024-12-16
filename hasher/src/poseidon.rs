mod compile_time;

mod impls;
pub use impls::{PoseidonParams, PoseidonState};

mod m31x16_ext3;
pub use m31x16_ext3::PoseidonM31x16Ext3;

#[cfg(test)]
mod tests;
