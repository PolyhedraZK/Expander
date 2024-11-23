mod utils;
pub use utils::{
    OrionCommitment, OrionPCSError, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    SubsetSumLUTs,
};

mod linear_code;
pub use linear_code::{OrionCodeParameter, ORION_CODE_PARAMETER_INSTANCE};

mod pcs_impl;
pub use pcs_impl::{orion_commit_base, orion_commit_simd_field, orion_open, orion_verify};

mod serde;

#[cfg(test)]
mod tests;
