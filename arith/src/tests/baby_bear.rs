use super::field::{random_field_tests, random_inversion_tests};
use crate::BabyBear;
use ark_std::test_rng;

#[test]
fn test_field() {
    random_field_tests::<BabyBear>("BabyBear".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<BabyBear, _>(&mut rng, "BabyBear".to_string());
}
