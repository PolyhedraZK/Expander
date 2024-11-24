mod utils;
pub use utils::{
    OrionCommitment, OrionPCSError, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    SubsetSumLUTs,
};

mod linear_code;
pub use linear_code::{OrionCodeParameter, ORION_CODE_PARAMETER_INSTANCE};

mod pcs_impl;
pub use pcs_impl::orion_verify;

mod base_field_impl;
pub use base_field_impl::{orion_commit_base_field, orion_open_base_field};

mod simd_field_impl;
pub use simd_field_impl::orion_commit_simd_field;

mod serde;

#[cfg(test)]
mod tests;
