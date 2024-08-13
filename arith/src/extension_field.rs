mod fr_ext;
mod gf2_128;
mod gf2_128x4;
mod m31_ext;
mod simd_m31_ext;
use crate::{Field, FieldSerde};

pub use gf2_128::*;
pub use gf2_128x4::GF2_128x4;
pub use m31_ext::M31Ext3;
pub use simd_m31_ext::SimdM31Ext3;

/// Configurations for Extension Field over
/// the Binomial polynomial x^DEGREE - W
//
// FIXME: Our binary extension field is no longer a binomial extension field
// will fix later
pub trait BinomialExtensionField: From<Self::BaseField> + Field + FieldSerde {
    /// Degree of the Extension
    const DEGREE: usize;

    /// Extension Field
    const W: u32;

    /// Base field for the extension
    type BaseField: Field + FieldSerde + Send;

    /// Multiply the extension field with the base field
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self;

    /// Add the extension field with the base field
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self;

    /// Get the basefield element from the extension field
    fn first_base_field(&self) -> Self::BaseField;
}
