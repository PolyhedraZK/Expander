use super::*;
use arith::{random_field_tests, random_inversion_tests, Field};
use ark_std::test_rng;
use ethnum::U256;
use rand::thread_rng;
use serdes::ExpSerde;

#[test]
fn test_base_field() {
    random_field_tests::<Goldilocks>("Goldilocks".to_string());

    let mut rng = test_rng();
    random_inversion_tests::<Goldilocks, _>(&mut rng, "Goldilocks".to_string());
}

#[test]
fn test_mod_reduction() {
    // Test reduction of values less than modulus
    assert_eq!(mod_reduce_u64(5), 5);

    // Test reduction of values equal to modulus
    assert_eq!(mod_reduce_u64(GOLDILOCKS_MOD), 0);

    // Test reduction of values greater than modulus
    assert_eq!(mod_reduce_u64(GOLDILOCKS_MOD + 1), 1);

    // Test reduction of 2^64 - 1
    assert_eq!(mod_reduce_u64(u64::MAX), EPSILON - 1);
}

#[test]
fn test_serialization() {
    let x = Goldilocks::random_unsafe(thread_rng());
    let mut bytes = Vec::new();
    x.serialize_into(&mut bytes).unwrap();
    assert_eq!(bytes.len(), Goldilocks::SERIALIZED_SIZE);

    let y = Goldilocks::deserialize_from(bytes.as_slice()).unwrap();
    assert_eq!(x, y);
}

#[test]
fn test_conversions() {
    // Test u32 conversion
    assert_eq!(Goldilocks::from(5u32).v, 5);

    // Test u64 conversion
    assert_eq!(Goldilocks::from(5u64).v, 5);

    // Test u256 conversion
    let u256 = U256::from(5u32);
    assert_eq!(Goldilocks::from_u256(u256).v, 5);
}

#[test]
fn test_exponentiation() {
    let x = Goldilocks::from(2u32);
    let y = x.exp(3);
    assert_eq!(y.v, 8);

    // Test power of 2
    let z = x.exp_power_of_2(2);
    assert_eq!(z.v, 16);
}

#[test]
fn test_special_values() {
    // Test INV_2
    assert_eq!(
        Goldilocks::INV_2 * Goldilocks::from(2u32),
        Goldilocks::one()
    );

    // Test mul_by_5
    let x = Goldilocks::from(2u32);
    assert_eq!(x.mul_by_5().v, 10);

    // Test mul_by_6
    assert_eq!(x.mul_by_6().v, 12);
}

#[test]
fn test_edge_cases() {
    // Test zero
    assert!(Goldilocks::zero().is_zero());

    // Test modulus
    assert!(Goldilocks { v: GOLDILOCKS_MOD }.is_zero());

    // Test inverse of zero
    assert!(Goldilocks::zero().inv().is_none());

    // Test large numbers
    let large = Goldilocks {
        v: GOLDILOCKS_MOD - 1,
    };
    assert_eq!(large + Goldilocks::one(), Goldilocks::zero());

    // Test U64::MAX; also tests Eq works over normalized values
    let max = Goldilocks { v: u64::MAX };
    let epsilon = Goldilocks { v: EPSILON - 1 };
    assert_eq!(max, epsilon);
}
