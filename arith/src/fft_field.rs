use crate::Field;

pub trait FFTField: Field {
    const TWO_ADICITY: u32;

    const ROOT_OF_UNITY: Self;

    /// Returns a generator of the multiplicative group of order `2^bits`.
    /// Assumes `bits < TWO_ADICITY`, otherwise the result is undefined.
    #[must_use]
    fn two_adic_generator(bits: usize) -> Self;
}
