use crate::Field;

pub trait FFTField: Field {
    const TWO_ADICITY: u32;

    const ROOT_OF_UNITY: Self;
}
