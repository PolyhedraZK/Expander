use std::io::Cursor;

use ark_std::test_rng;

use crate::{FieldSerde, SimdGF2, GF2};

use super::{
    field::{random_field_tests, random_inversion_tests},
    simd_field::random_simd_field_tests,
};

#[test]
fn test_field() {
    random_field_tests::<GF2>("GF2".to_string());
    random_field_tests::<SimdGF2>("Vectorized GF2".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<GF2, _>(&mut rng, "GF2".to_string());
    random_simd_field_tests::<SimdGF2>("Vectorized GF2".to_string());
}

#[test]
fn test_custom_serde_vectorize_gf2() {
    let a = SimdGF2::from(0);
    let mut buffer = vec![];
    a.serialize_into(&mut buffer);
    let mut cursor = Cursor::new(buffer);
    let b = SimdGF2::deserialize_from(&mut cursor);
    assert_eq!(a, b);
}
