mod baby_bear_ext;
mod fr_ext;
// mod gf2_127;
mod gf2_128;
mod gf2_128x8;
mod m31_ext;
mod m31_ext3x16;
use crate::{Field, FieldSerde};

pub use baby_bear_ext::*;
// pub use gf2_127::*;
pub use gf2_128::*;
pub use gf2_128x8::GF2_128x8;
#[cfg(target_arch = "x86_64")]
pub use gf2_128x8::GF2_128x8_256;
pub use m31_ext::M31Ext3;
pub use m31_ext3x16::M31Ext3x16;

/// Configurations for Extension Field over
/// - either the Binomial polynomial x^DEGREE - W
/// - or the AES polynomial x^128 + x^7 + x^2 + x + 1
//
pub trait ExtensionField: From<Self::BaseField> + Field + FieldSerde {
    /// Degree of the Extension
    const DEGREE: usize;

    /// constant term if the extension field is represented as a binomial polynomial
    const W: u32;

    /// x, i.e, 0 + x + 0 x^2 + 0 x^3 + ...
    const X: Self;

    /// Base field for the extension
    type BaseField: Field + FieldSerde + Send;

    /// Multiply the extension field with the base field
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self;

    /// Add the extension field with the base field
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self;

    /// Multiply the extension field element by x, i.e, 0 + x + 0 x^2 + 0 x^3 + ...
    fn mul_by_x(&self) -> Self;
}
