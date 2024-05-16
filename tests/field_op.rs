use expander_rs::{Field, PackedM31, VectorizedM31, M31};
use rand::prelude::*;

fn test_basic_field_op<F: Field>() {
    let f = F::random();
    let mut rng = rand::thread_rng();
    let rhs = rng.gen::<usize>() % 100;

    let prod_0 = f * F::from(rhs);
    let mut prod_1 = F::zero();
    for _ in 0..rhs {
        prod_1 += f;
    }
    assert_eq!(prod_0, prod_1);

    let f_inv = f.inv();
    assert_eq!(f * f_inv, F::one());
}

#[test]
fn test_m31_basic_field_op() {
    test_basic_field_op::<M31>();
}

#[test]
fn test_packed_m31_basic_field_op() {
    test_basic_field_op::<PackedM31>();
}

#[test]
fn test_vectorize_m31_basic_field_op() {
    test_basic_field_op::<VectorizedM31>();
}
