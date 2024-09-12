use ark_std::test_rng;
use halo2curves::bn256::Fr;
use halo2curves::ff::Field as Halo2Field;
use rand::RngCore;

use super::field::{random_field_tests, random_inversion_tests};
use crate::Field;

#[test]
fn test_field() {
    random_field_tests::<Fr>("bn254::Fr".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<Fr, _>(&mut rng, "bn254::Fr".to_string());
}

#[test]
fn test_mul_by_i32() {
    let mut rng = test_rng();
    let a = Fr::random(&mut rng);
    // let a = Fr::one();
    let b = test_rng().next_u32() as i32;
    // let b = 1;
    // let b = -2;

    let b_fr = if b < 0 {
        Fr::from((-b) as u64).neg()
    } else {
        Fr::from(b as u64)
    };

    let c = a.mul_by_i32(b);
    let c2 = a * b_fr;
    // unsafe {
    //     assert_eq!(
    //         std::mem::transmute::<_, [u64; 4]>(c),
    //         std::mem::transmute::<_, [u64; 4]>(c2)
    //     );
    // }
    assert_eq!(c, c2);
}
