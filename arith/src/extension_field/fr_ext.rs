use halo2curves::bn256::Fr;

use super::BinomialExtensionField;

impl BinomialExtensionField for Fr {
    const DEGREE: usize = 1;

    /// Extension Field over X-1 which is self
    const W: u32 = 1;

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

    /// Get the basefield element from the extension field
    fn first_base_field(&self) -> Self::BaseField {
        *self
    }
}
