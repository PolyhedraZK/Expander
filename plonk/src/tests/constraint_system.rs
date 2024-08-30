use arith::Field;
use halo2curves::{
    bn256::Fr,
    ff::{PrimeField, WithSmallOrderMulGroup},
};

use crate::{constraint_system::ConstraintSystem, PlonkIOP};

#[test]
fn test_gates() {
    test_suites::<Fr>();
}

fn test_suites<F: Field + PrimeField + WithSmallOrderMulGroup<3>>() {
    test_arithmetics_helper::<F>();
    test_boolean_helper::<F>();
    test_equality_helper::<F>();
    test_selection_helper::<F>();
    test_pi_helper::<F>();
    test_h_poly::<F>();
}

fn test_arithmetics_helper<F: Field + PrimeField + WithSmallOrderMulGroup<3>>() {
    {
        // addition gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(2u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.addition_gate(&a, &b);
        assert_eq!(cs.get_value(c), F::from(5u64));
        cs.assert_addition(&a, &b, &c);

        assert!(cs.is_satisfied(&[]));

        let d = cs.new_variable(F::from(7u64));
        cs.assert_addition(&a, &b, &d);

        assert!(!cs.is_satisfied(&[]));
    }

    {
        // subtraction gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(2u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.subtraction_gate(&a, &b);
        assert_eq!(cs.get_value(c), -F::from(1u64));
        cs.assert_subtraction(&a, &b, &c);

        assert!(cs.is_satisfied(&[]));

        let d = cs.new_variable(F::from(7u64));
        cs.assert_subtraction(&a, &b, &d);

        assert!(!cs.is_satisfied(&[]));
    }

    {
        // multiplication gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(2u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.multiplication_gate(&a, &b);
        assert_eq!(cs.get_value(c), F::from(6u64));
        cs.assert_multiplication(&a, &b, &c);

        assert!(cs.is_satisfied(&[]));

        let d = cs.new_variable(F::from(7u64));
        cs.assert_multiplication(&a, &b, &d);

        assert!(!cs.is_satisfied(&[]));
    }

    {
        // division gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(6u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.division_gate(&a, &b);

        assert_eq!(cs.get_value(c), F::from(2u64));

        cs.assert_division(&a, &b, &c);

        assert!(cs.is_satisfied(&[]));

        let d = cs.new_variable(F::from(7u64));
        cs.assert_division(&a, &b, &d);
    }
}

fn test_boolean_helper<F: Field + PrimeField + WithSmallOrderMulGroup<3>>() {
    let mut cs = ConstraintSystem::<F>::init();

    // assert one
    let a = cs.new_variable(F::from(1u64));
    cs.assert_one(&a);

    // assert zero
    let b = cs.new_variable(F::from(0u64));
    cs.assert_zero(&b);

    // assert binary
    cs.assert_binary(&a);
    cs.assert_binary(&b);

    // assert none zero
    let c = cs.new_variable(F::from(2u64));
    cs.assert_nonzero(&c);

    assert!(cs.is_satisfied(&[]));
}

fn test_equality_helper<F: Field + PrimeField + WithSmallOrderMulGroup<3>>() {
    let mut cs = ConstraintSystem::<F>::init();

    let a = cs.new_variable(F::from(2u64));
    let b = cs.new_variable(F::from(2u64));
    cs.assert_equal(&a, &b);

    assert!(cs.is_satisfied(&[]));

    let c = cs.new_variable(F::from(3u64));
    cs.assert_equal(&a, &c);

    assert!(!cs.is_satisfied(&[]));
}

fn test_selection_helper<F: Field + PrimeField + WithSmallOrderMulGroup<3>>() {
    let mut cs = ConstraintSystem::<F>::init();

    let first = cs.new_variable(F::from(0u64));
    let second = cs.new_variable(F::from(1u64));

    let a = cs.new_variable(F::from(3u64));
    let b = cs.new_variable(F::from(4u64));

    let a_selected = cs.selection_gate(&first, &a, &b);
    let b_selected = cs.selection_gate(&second, &a, &b);

    cs.assert_equal(&a, &a_selected);
    cs.assert_equal(&b, &b_selected);

    assert!(cs.is_satisfied(&[]));
}

fn test_pi_helper<F: Field + PrimeField + WithSmallOrderMulGroup<3>>() {
    let mut cs = ConstraintSystem::<F>::init();
    let two = F::from(2u64);
    cs.public_input_gate(two);

    assert!(cs.is_satisfied(&[two]));
    assert!(!cs.is_satisfied(&[F::one()]));
}

fn test_h_poly<F: Field + PrimeField + WithSmallOrderMulGroup<3>>() {
    let mut cs = ConstraintSystem::<F>::init();
    {
        let a = cs.new_variable(F::from(2u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.addition_gate(&a, &b);
        assert_eq!(cs.get_value(c), F::from(5u64));
        cs.assert_addition(&a, &b, &c);
        let c = cs.addition_gate(&a, &b);
        assert_eq!(cs.get_value(c), F::from(5u64));
        cs.assert_addition(&a, &b, &c);
    }

    {
        let a = cs.new_variable(F::from(2u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.subtraction_gate(&a, &b);
        assert_eq!(cs.get_value(c), -F::from(1u64));
        cs.assert_subtraction(&a, &b, &c);
    }

    {
        let a = cs.new_variable(F::from(2u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.multiplication_gate(&a, &b);
        assert_eq!(cs.get_value(c), F::from(6u64));
        cs.assert_multiplication(&a, &b, &c);
    }

    {
        let a = cs.new_variable(F::from(6u64));
        let b = cs.new_variable(F::from(3u64));
        let c = cs.division_gate(&a, &b);

        cs.assert_division(&a, &b, &c);
    }

    {
        let first = cs.new_variable(F::from(0u64));
        let second = cs.new_variable(F::from(1u64));

        let a = cs.new_variable(F::from(3u64));
        let b = cs.new_variable(F::from(4u64));

        let a_selected = cs.selection_gate(&first, &a, &b);
        let b_selected = cs.selection_gate(&second, &a, &b);

        cs.assert_equal(&a, &a_selected);
        cs.assert_equal(&b, &b_selected);
    }

    assert!(cs.is_satisfied(&[]));
    cs.finalize();

    println!("cs: number of witnesses: {:?}", cs.witness_list.witnesses.len());

    let _hx = PlonkIOP::generate_zero_polynomial(&cs, &[]);

    // none zero terms in hx
    let non_zero_in_h = _hx.iter().filter(|x| !x.is_zero_vartime()).count();
    println!("degree of h: {:?}", non_zero_in_h);
}
