use ark_std::test_rng;
use std::io::Cursor;

use arith::{
    random_extension_field_tests, random_field_tests, random_inversion_tests,
    random_simd_field_tests, FieldSerde,
};

use crate::{GF2_128x8, GF2_128};

#[test]
fn test_simd_field() {
    random_simd_field_tests::<GF2_128x8>("Simd GF2 Ext128".to_string());
}

#[test]
fn test_ext_field() {
    random_field_tests::<GF2_128>("GF2 Ext128".to_string());
    random_field_tests::<GF2_128x8>("Simd GF2 Ext128".to_string());

    random_extension_field_tests::<GF2_128>("GF2 Ext128".to_string());
    random_extension_field_tests::<GF2_128x8>("Simd GF2 Ext128".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<GF2_128, _>(&mut rng, "GF2_128".to_string());
}

#[test]
fn test_custom_serde_vectorize_gf2_128() {
    let a = GF2_128::from(0);
    let mut buffer = vec![];
    assert!(a.serialize_into(&mut buffer).is_ok());
    let mut cursor = Cursor::new(buffer);
    let b = GF2_128::deserialize_from(&mut cursor);
    assert!(b.is_ok());
    let b = b.unwrap();
    assert_eq!(a, b);
}
