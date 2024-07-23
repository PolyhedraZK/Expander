use crate::M31Ext3;

#[cfg(target_arch = "x86_64")]
use crate::VectorizedM31Ext3;

use super::field::random_field_tests;
#[test]
fn test_field() {
    random_field_tests::<M31Ext3>("M31 Ext3".to_string());

    #[cfg(target_arch = "x86_64")]
    random_field_tests::<VectorizedM31Ext3>("Vectorized M31 Ext3".to_string());
}

// #[test]
// fn test_mul_by_base() {
//     let mut rng = rand::thread_rng();
//     let a = M31Ext3::random_unsafe(&mut rng);
//     let b = <M31Ext3 as Field>::BaseField::random_unsafe(&mut rng);
//     let c = M31Ext3::random_unsafe(&mut rng);
//     let d = <M31Ext3 as Field>::BaseField::random_unsafe(&mut rng);

//     {
//         let mut t0 = a; // (a * b) * c
//         t0 = t0.mul_base_elem(&b);
//         t0.mul_assign(&c);

//         let mut t1 = a; // (a * c) * b
//         t1.mul_assign(&c);
//         t1 = t1.mul_base_elem(&b);

//         let mut t2 = c; // (b * c) * a
//         t2.mul_assign_base_elem(&b);
//         t2.mul_assign(&a);

//         assert_eq!(t0, t1);
//         assert_eq!(t1, t2);
//     }

//     {
//         let mut t0 = a; // (a * b) * d
//         t0 = t0.mul_base_elem(&b);
//         t0 = t0.mul_base_elem(&d);

//         let mut t1 = a; // (a * d) * b
//         t1 = t1.mul_base_elem(&d);
//         t1 = t1.mul_base_elem(&b);

//         let mut t2 = d; // (b * d) * a
//         t2.mul_assign_base_elem(&b);
//         let t2 = a.mul_base_elem(&t2);

//         assert_eq!(t0, t1);
//         assert_eq!(t0, t2);
//     }
// }
