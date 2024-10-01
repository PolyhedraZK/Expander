use halo2curves::bn256::Fr;

use super::ExtensionField;

impl ExtensionField for Fr {
    const DEGREE: usize = 1;

    /// Extension Field over X-1 which is self
    const W: u32 = 1;

    // placeholder, doesn't make sense for Fr
    const X: Self = Fr::zero();

    /// Base field for the extension
    type BaseField = Self;

    /// Multiply the extension field with the base field
    fn mul_by_base_field(&self, base: &Self::BaseField) -> Self {
        self * base
    }

    /// Add the extension field with the base field
    fn add_by_base_field(&self, base: &Self::BaseField) -> Self {
        self + base
    }

    /// Multiply the extension field by x, i.e, 0 + x + 0 x^2 + 0 x^3 + ...
    fn mul_by_x(&self) -> Self {
        unimplemented!("mul_by_x for Fr doesn't make sense")
    }
}
