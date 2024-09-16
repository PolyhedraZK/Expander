use super::{
    field::{random_field_tests, random_inversion_tests},
    simd_field::random_simd_field_tests,
};
use crate::{baby_bear_avx::AVXBabyBear, BabyBear};
use ark_std::test_rng;

// CMD: RUSTFLAGS="-C target-feature=+avx512f" cargo test --package arith --lib -- tests::baby_bear::test_field --exact --show-output
#[test]
fn test_field() {
    random_field_tests::<BabyBear>("BabyBear".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<BabyBear, _>(&mut rng, "BabyBear".to_string());
    random_inversion_tests::<AVXBabyBear, _>(&mut rng, "Vectorized BabyBear".to_string());

    random_simd_field_tests::<AVXBabyBear>("Vectorized BabyBear".to_string());
}
