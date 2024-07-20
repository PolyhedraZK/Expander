use std::io::Cursor;

use ark_std::{end_timer, start_timer, test_rng};
use rand::{Rng, RngCore};

use crate::{ExtensionField, Field, FieldSerde, VectorizedField};

pub(crate) fn test_basic_field_op<F: Field>() {
    let mut rng = rand::thread_rng();

    let f = F::random_unsafe(&mut rng);

    let rhs = rng.gen::<u32>() % 100;

    let prod_0 = f * F::from(rhs);
    let mut prod_1 = F::zero();
    for _ in 0..rhs {
        prod_1 += f;
    }
    assert_eq!(prod_0, prod_1);
}

pub(crate) fn random_extension_field_tests<F: ExtensionField>(type_name: String) {
    let mut rng = test_rng();

    let _message = format!("multiplication {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let a = F::random_unsafe(&mut rng);
        let b = F::BaseField::random_unsafe(&mut rng);
        let c = F::random_unsafe(&mut rng);

        let mut t0 = a; // (a * b) * c
        t0 *= b;
        t0 *= c;

        let mut t1 = a; // (a * c) * b
        t1 *= c;
        t1 *= b;

        let mut t2 = c; // (b * c) * a
        t2 *= b;
        t2 *= a;

        assert_eq!(t0, t1);
        assert_eq!(t1, t2);
    }
    end_timer!(start);
}

pub fn random_field_tests<F: Field>(type_name: String) {
    let mut rng = test_rng();

    random_multiplication_tests::<F, _>(&mut rng, type_name.clone());
    random_addition_tests::<F, _>(&mut rng, type_name.clone());
    random_subtraction_tests::<F, _>(&mut rng, type_name.clone());
    random_negation_tests::<F, _>(&mut rng, type_name.clone());
    random_doubling_tests::<F, _>(&mut rng, type_name.clone());
    random_squaring_tests::<F, _>(&mut rng, type_name.clone());
    // random_inversion_tests::<F, _>(&mut rng, type_name.clone());
    random_expansion_tests::<F, _>(&mut rng, type_name);

    assert_eq!(F::zero().is_zero(), true);
    {
        let mut z = F::zero();
        z = z.neg();
        assert_eq!(z.is_zero(), true);
    }

    // assert!(bool::from(F::zero().inv().is_none()));

    // Multiplication by zero
    {
        let mut a = F::random_unsafe(&mut rng);
        a.mul_assign(&F::zero());
        assert_eq!(a.is_zero(), true);
    }

    // Addition by zero
    {
        let mut a = F::random_unsafe(&mut rng);
        let copy = a;
        a.add_assign(&F::zero());
        assert_eq!(a, copy);
    }
}

fn random_multiplication_tests<F: Field, R: RngCore>(mut rng: R, type_name: String) {
    let _message = format!("multiplication {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let a = F::random_unsafe(&mut rng);
        let b = F::random_unsafe(&mut rng);
        let c = F::random_unsafe(&mut rng);

        let mut t0 = a; // (a * b) * c
        t0.mul_assign(&b);
        t0.mul_assign(&c);

        let mut t1 = a; // (a * c) * b
        t1.mul_assign(&c);
        t1.mul_assign(&b);

        let mut t2 = b; // (b * c) * a
        t2.mul_assign(&c);
        t2.mul_assign(&a);

        assert_eq!(t0, t1);
        assert_eq!(t1, t2);
        assert_eq!(a.square(), a * a);
    }
    end_timer!(start);
}

fn random_addition_tests<F: Field, R: RngCore>(mut rng: R, type_name: String) {
    let _message = format!("addition {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let a = F::random_unsafe(&mut rng);
        let b = F::random_unsafe(&mut rng);
        let c = F::random_unsafe(&mut rng);

        let mut t0 = a; // (a + b) + c
        t0.add_assign(&b);
        t0.add_assign(&c);

        let mut t1 = a; // (a + c) + b
        t1.add_assign(&c);
        t1.add_assign(&b);

        let mut t2 = b; // (b + c) + a
        t2.add_assign(&c);
        t2.add_assign(&a);

        assert_eq!(t0, t1);
        assert_eq!(t1, t2);
    }
    end_timer!(start);
}

fn random_subtraction_tests<F: Field, R: RngCore>(mut rng: R, type_name: String) {
    let _message = format!("subtraction {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let a = F::random_unsafe(&mut rng);
        let b = F::random_unsafe(&mut rng);

        let mut t0 = a; // (a - b)
        t0.sub_assign(&b);

        let mut t1 = b; // (b - a)
        t1.sub_assign(&a);

        let mut t2 = t0; // (a - b) + (b - a) = 0
        t2.add_assign(&t1);

        assert_eq!(t2.is_zero(), true);
    }
    end_timer!(start);
}

fn random_negation_tests<F: Field, R: RngCore>(mut rng: R, type_name: String) {
    let _message = format!("negation {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let a = F::random_unsafe(&mut rng);
        let mut b = a;
        b = b.neg();
        b.add_assign(&a);

        assert_eq!(b.is_zero(), true);
    }
    end_timer!(start);
}

fn random_doubling_tests<F: Field, R: RngCore>(mut rng: R, type_name: String) {
    let _message = format!("doubling {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let mut a = F::random_unsafe(&mut rng);
        let mut b = a;
        a.add_assign(&b);
        b = b.double();

        assert_eq!(a, b);
    }
    end_timer!(start);
}

fn random_squaring_tests<F: Field, R: RngCore>(mut rng: R, type_name: String) {
    let _message = format!("squaring {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let mut a = F::random_unsafe(&mut rng);
        let mut b = a;
        a.mul_assign(&b);
        b = b.square();

        assert_eq!(a, b);
    }
    end_timer!(start);
}

pub fn random_inversion_tests<F: Field>(type_name: String) {
    let mut rng = test_rng();

    assert!(bool::from(F::zero().inv().is_none()));

    let _message = format!("inversion {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        let mut a = F::random_unsafe(&mut rng);
        let b = a.inv().unwrap(); // probabilistically nonzero
        a.mul_assign(&b);
        assert_eq!(a, F::one());
    }
    end_timer!(start);
}

fn random_expansion_tests<F: Field, R: RngCore>(mut rng: R, type_name: String) {
    let _message = format!("expansion {}", type_name);
    let start = start_timer!(|| _message);
    for _ in 0..1000 {
        // Compare (a + b)(c + d) and (a*c + b*c + a*d + b*d)

        let a = F::random_unsafe(&mut rng);
        let b = F::random_unsafe(&mut rng);
        let c = F::random_unsafe(&mut rng);
        let d = F::random_unsafe(&mut rng);

        let mut t0 = a;
        t0.add_assign(&b);
        let mut t1 = c;
        t1.add_assign(&d);
        t0.mul_assign(&t1);

        let mut t2 = a;
        t2.mul_assign(&c);
        let mut t3 = b;
        t3.mul_assign(&c);
        let mut t4 = a;
        t4.mul_assign(&d);
        let mut t5 = b;
        t5.mul_assign(&d);

        t2.add_assign(&t3);
        t2.add_assign(&t4);
        t2.add_assign(&t5);

        assert_eq!(t0, t2);
    }
    end_timer!(start);
}

// pub fn random_vectorized_field_tests<F: VectorizedField + FieldSerde>(type_name: String) {
//     let mut rng = test_rng();

//     random_serdes_tests::<F, _>(&mut rng, type_name);
// }

// fn random_serdes_tests<F: VectorizedField + FieldSerde, R: RngCore>(
//     mut rng: R,
//     _type_name: String,
// ) {
//     let start = start_timer!(|| format!("expansion {}", _type_name));
//     for _ in 0..100 {
//         // convert a into and from bytes

//         let a = F::random_unsafe(&mut rng);
//         let mut buffer = vec![];
//         a.serialize_into(&mut buffer);
//         let mut cursor = Cursor::new(buffer);
//         let b = F::deserialize_from(&mut cursor);
//         assert_eq!(a, b);
//     }

//     let a = (0..100)
//         .map(|_| F::random_unsafe(&mut rng))
//         .collect::<Vec<_>>();
//     let mut buffer = vec![];
//     a.iter().for_each(|x| x.serialize_into(&mut buffer));
//     let mut cursor = Cursor::new(buffer);
//     let b = (0..100)
//         .map(|_| F::deserialize_from(&mut cursor))
//         .collect::<Vec<_>>();
//     assert_eq!(a, b);

//     end_timer!(start);
// }
