use crate::M31Ext3;
use crate::SimdM31Ext3;

use super::{
    extension_field::random_extension_field_tests, field::random_field_tests,
    simd_field::random_simd_field_tests,
};
#[test]
fn test_field() {
    random_field_tests::<M31Ext3>("M31 Ext3".to_string());
    random_extension_field_tests::<M31Ext3>("M31 Ext3".to_string());

    random_field_tests::<SimdM31Ext3>("Simd M31 Ext3".to_string());
    random_extension_field_tests::<SimdM31Ext3>("Simd M31 Ext3".to_string());
    random_simd_field_tests::<SimdM31Ext3>("Simd M31 Ext3".to_string());
}
