use std::io::Cursor;

use ark_std::test_rng;

use crate::{FieldSerde, VectorizedM31, M31};

use super::field::{random_field_tests, random_inversion_tests, test_basic_field_op};

#[test]
fn test_field() {
    random_field_tests::<M31>("M31".to_string());
    random_field_tests::<VectorizedM31>("Vectorized M31".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<M31, _>(&mut rng, "M31".to_string());
    random_inversion_tests::<VectorizedM31, _>(&mut rng, "Vectorized M31".to_string());
}

#[test]
fn test_m31_basic_field_op() {
    test_basic_field_op::<M31>();
}

#[test]
fn test_vectorize_m31_basic_field_op() {
    test_basic_field_op::<VectorizedM31>();
}

#[test]
fn test_custom_serde_vectorize_m31() {
    let a = VectorizedM31::from(256 + 2);
    let mut buffer = vec![];
    a.serialize_into(&mut buffer);
    let mut cursor = Cursor::new(buffer);
    let b = VectorizedM31::deserialize_from(&mut cursor);
    assert_eq!(a, b);
}
