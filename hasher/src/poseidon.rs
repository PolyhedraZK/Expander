mod compile_time;

mod impls;
pub use impls::{PoseidonParams, PoseidonSponge, PoseidonState};

mod m31x16_ext3;
pub use m31x16_ext3::PoseidonM31x16Ext3;

mod aliases;
pub use aliases::PoseidonM31TranscriptSponge;

#[cfg(test)]
mod tests;
