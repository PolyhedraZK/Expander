use arith::{random_field_tests, random_inversion_tests};
use ark_std::test_rng;

use crate::BabyBear;



#[test]
fn test_base_field() {
    random_field_tests::<BabyBear>("BabyBear".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<BabyBear, _>(&mut rng, "BabyBear".to_string());
}
