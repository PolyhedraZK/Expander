use std::io::Cursor;

use ark_std::test_rng;

use crate::{FieldSerde, SimdGF2, SimdM31, GF2, M31};

use super::{
    field::{random_field_tests, random_inversion_tests},
    simd_field::random_simd_field_tests,
};

#[test]
fn test_field() {
    random_field_tests::<GF2>("M31".to_string());
    random_field_tests::<SimdGF2>("Vectorized M31".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<GF2, _>(&mut rng, "M31".to_string());
    random_inversion_tests::<SimdGF2, _>(&mut rng, "Vectorized M31".to_string());

    random_simd_field_tests::<SimdGF2>("Vectorized M31".to_string());
}

#[test]
fn test_custom_serde_gf2() {}
