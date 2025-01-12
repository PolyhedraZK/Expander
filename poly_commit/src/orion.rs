mod utils;
pub use utils::{
    OrionCommitment, OrionPCSError, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    SubsetSumLUTs,
};

mod linear_code;
pub use linear_code::{OrionCodeParameter, ORION_CODE_PARAMETER_INSTANCE};

#[cfg(test)]
mod linear_code_tests;

mod base_field_impl;
pub use base_field_impl::{
    orion_commit_base_field, orion_open_base_field, orion_verify_base_field,
};

#[cfg(test)]
mod base_field_tests;

mod simd_field_impl;
pub use simd_field_impl::{
    orion_commit_simd_field, orion_open_simd_field, orion_verify_simd_field,
};

#[cfg(test)]
mod simd_field_tests;

mod simd_field_agg_impl;

#[cfg(test)]
mod simd_field_agg_tests;

mod pcs_for_expander_gkr;
pub use pcs_for_expander_gkr::OrionPCSForGKR;

mod pcs_trait_impl;
pub use pcs_trait_impl::{OrionBaseFieldPCS, OrionSIMDFieldPCS};

mod serde;
