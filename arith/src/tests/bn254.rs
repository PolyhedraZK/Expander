use std::io::Cursor;

use crate::{FieldSerde, VectorizedFr};
use halo2curves::bn256::Fr;

use super::field::{
    random_field_tests, random_inversion_tests, random_small_field_tests,
    random_vectorized_field_tests, test_basic_field_op,
};

#[test]
fn test_field() {
    random_field_tests::<Fr>("bn254::Fr".to_string());
    random_inversion_tests::<Fr>("bn254::Fr".to_string());
    random_small_field_tests::<Fr>("bn254::Fr".to_string());

    random_vectorized_field_tests::<VectorizedFr>("Vectorized M31".to_string());
}

#[test]
fn test_bn254_basic_field_op() {
    test_basic_field_op::<Fr>();
}

#[test]
fn test_packed_bn254_basic_field_op() {
    test_basic_field_op::<Fr>();
}

#[test]
fn test_vectorize_bn254_basic_field_op() {
    test_basic_field_op::<VectorizedFr>();
}

#[test]
fn test_custom_serde_vectorize_bn254() {
    let a = VectorizedFr::from(256 + 2);
    let mut buf = vec![];
    a.serialize_into(&mut buf);
    let mut cursor = Cursor::new(buf);
    let b = VectorizedFr::deserialize_from(&mut cursor);
    assert_eq!(a, b);
}
