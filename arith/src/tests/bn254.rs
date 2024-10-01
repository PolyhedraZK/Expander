use ark_std::test_rng;
use halo2curves::bn256::Fr;

use super::field::{random_field_tests, random_inversion_tests};

#[test]
fn test_field() {
    random_field_tests::<Fr>("bn254::Fr".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<Fr, _>(&mut rng, "bn254::Fr".to_string());
}
