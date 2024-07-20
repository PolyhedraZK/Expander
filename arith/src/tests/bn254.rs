use halo2curves::bn256::Fr;

use super::field::{
    random_field_tests, //random_inversion_tests,
    test_basic_field_op,
};

#[test]
fn test_field() {
    random_field_tests::<Fr>("bn254::Fr".to_string());
    // random_inversion_tests::<Fr>("bn254::Fr".to_string());
}

#[test]
fn test_bn254_basic_field_op() {
    test_basic_field_op::<Fr>();
}
