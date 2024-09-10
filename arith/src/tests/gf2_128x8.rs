use crate::GF2_128x8;
use crate::GF2_128;

use super::{
    extension_field::random_extension_field_tests, field::random_field_tests,
    simd_field::random_simd_field_tests,
};
#[test]
fn test_field() {
    random_field_tests::<GF2_128>("GF2 Ext128".to_string());
    random_extension_field_tests::<GF2_128>("GF2 Ext128".to_string());

    random_field_tests::<GF2_128x8>("Simd GF2 Ext128".to_string());
    random_extension_field_tests::<GF2_128x8>("Simd GF2 Ext128".to_string());
    random_simd_field_tests::<GF2_128x8>("Simd GF2 Ext128".to_string());
}
