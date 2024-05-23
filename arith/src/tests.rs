use crate::{Field, PackedM31, VectorizedM31, M31_VECTORIZE_SIZE, VECTORIZEDM31_INV_2};
use rand::prelude::*;

#[cfg(target_arch = "x86_64")]
#[test]
fn test_mm256_const_init() {
    use std::arch::x86_64::*;
    use std::mem::transmute;

    let x = unsafe { _mm256_set1_epi32(1) };
    println!("{:?}", x);
    pub const Y: __m256i = unsafe { transmute([1, 1, 1, 1, 1, 1, 1, 1]) };
    println!("{:?}", Y);
}

fn test_basic_field_op<F: Field>() {
    let f = F::random();
    let mut rng = rand::thread_rng();
    let rhs = rng.gen::<u32>() % 100;

    let prod_0 = f * F::from(rhs);
    let mut prod_1 = F::zero();
    for _ in 0..rhs {
        prod_1 += f;
    }
    assert_eq!(prod_0, prod_1);

    // let f_inv = f.inv();
    // assert_eq!(f * f_inv, F::one());
}

// #[test]
// fn test_m31_basic_field_op() {
//     test_basic_field_op::<M31>();
// }

#[test]
fn test_packed_m31_basic_field_op() {
    test_basic_field_op::<PackedM31>();
}

#[test]
fn test_vectorize_m31_basic_field_op() {
    test_basic_field_op::<VectorizedM31>();
}

#[test]
fn test_sanity_check_vectorize_m31() {
    let mut a = VectorizedM31::from(1);
    let b = VectorizedM31::from(2);
    a += b;
    assert_eq!(a, VectorizedM31::from(3));
    assert_eq!(b * VECTORIZEDM31_INV_2, VectorizedM31::from(1));
    assert_eq!(b * b * VECTORIZEDM31_INV_2, b);
}

#[test]
fn test_custom_serde_vectorize_m31() {
    let a = VectorizedM31::from(256 + 2);
    let mut buffer = vec![PackedM31::default(); M31_VECTORIZE_SIZE];
    let buffer_slice: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(
            buffer.as_mut_ptr() as *mut u8,
            buffer.len() * std::mem::size_of::<PackedM31>(),
        )
    };
    a.serialize_into(buffer_slice);
    println!("{:?}", buffer_slice);
    let b = VectorizedM31::deserialize_from(&buffer_slice);
    assert_eq!(a, b);
}
