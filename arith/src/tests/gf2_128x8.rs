use crate::GF2_128;
use crate::GF2_128x8;

use super::{
    extension_field::random_extension_field_tests, field::random_field_tests,
    simd_field::random_simd_field_tests,
};
#[test]
fn test_field() {
    random_field_tests::<GF2_128>("M31 Ext3".to_string());
    random_extension_field_tests::<GF2_128>("M31 Ext3".to_string());

    random_field_tests::<GF2_128x8>("Simd M31 Ext3".to_string());
    random_extension_field_tests::<GF2_128x8>("Simd M31 Ext3".to_string());
    random_simd_field_tests::<GF2_128x8>("Simd M31 Ext3".to_string());
}
