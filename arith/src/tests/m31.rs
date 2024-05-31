use crate::{FieldSerde, PackedM31, VectorizedM31, M31, M31_VECTORIZE_SIZE, VECTORIZEDM31_INV_2};

use super::field::{
    random_field_tests, random_inversion_tests, random_small_field_tests,
    random_vectorized_field_tests, test_basic_field_op,
};

#[test]
fn test_field() {
    random_field_tests::<M31>("M31".to_string());
    random_inversion_tests::<M31>("M31".to_string());
    random_small_field_tests::<M31>("M31".to_string());

    random_field_tests::<VectorizedM31>("Vectorized M31".to_string());
    random_small_field_tests::<VectorizedM31>("Vectorized M31".to_string());

    random_vectorized_field_tests::<VectorizedM31>("Vectorized M31".to_string());
}

#[test]
fn test_m31_basic_field_op() {
    test_basic_field_op::<M31>();
}

#[test]
fn test_packed_m31_basic_field_op() {
    test_basic_field_op::<PackedM31>();
}

#[test]
fn test_vectorize_m31_basic_field_op() {
    test_basic_field_op::<VectorizedM31>();
}

#[test]
fn test_sanity_check_vectorize_m31() {
    let mut a = VectorizedM31::from(1);
    let b = VectorizedM31::from(2);
    a += b;
    assert_eq!(a, VectorizedM31::from(3));
    assert_eq!(b * VECTORIZEDM31_INV_2, VectorizedM31::from(1));
    assert_eq!(b * b * VECTORIZEDM31_INV_2, b);
}

#[test]
fn test_custom_serde_vectorize_m31() {
    let a = VectorizedM31::from(256 + 2);
    let mut buffer = vec![PackedM31::default(); M31_VECTORIZE_SIZE];
    let buffer_slice: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(
            buffer.as_mut_ptr() as *mut u8,
            buffer.len() * std::mem::size_of::<PackedM31>(),
        )
    };
    a.serialize_into(buffer_slice);
    println!("{:?}", buffer_slice);
    let b = VectorizedM31::deserialize_from(&buffer_slice);
    assert_eq!(a, b);
}
