use arith::{Field, M31};
use halo2curves::bn256::Fr;

use crate::constraint_system::ConstraintSystem;

use super::*;

#[test]
fn test_constraint_system() {
    test_suites::<Fr>();
    test_suites::<M31>();
}

fn test_suites<F: Field>() {
    test_arithmetics_helper::<F>();
    test_boolean_helper::<F>();
    test_equality_helper::<F>();
}

fn test_arithmetics_helper<F: Field>() {
    {
        // addition gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(2));
        let b = cs.new_variable(F::from(3));
        let c = cs.addition_gate(&a, &b);
        assert_eq!(cs.get_value(c), F::from(5));
        cs.assert_addition(&a, &b, &c);

        assert!(cs.is_satisfied());

        let d = cs.new_variable(F::from(7));
        cs.assert_addition(&a, &b, &d);

        assert!(!cs.is_satisfied());
    }

    {
        // subtraction gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(2));
        let b = cs.new_variable(F::from(3));
        let c = cs.subtraction_gate(&a, &b);
        assert_eq!(cs.get_value(c), -F::from(1));
        cs.assert_subtraction(&a, &b, &c);

        assert!(cs.is_satisfied());

        let d = cs.new_variable(F::from(7));
        cs.assert_subtraction(&a, &b, &d);

        assert!(!cs.is_satisfied());
    }

    {
        // multiplication gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(2));
        let b = cs.new_variable(F::from(3));
        let c = cs.multiplication_gate(&a, &b);
        assert_eq!(cs.get_value(c), F::from(6));
        cs.assert_multiplication(&a, &b, &c);

        assert!(cs.is_satisfied());

        let d = cs.new_variable(F::from(7));
        cs.assert_multiplication(&a, &b, &d);

        assert!(!cs.is_satisfied());
    }

    {
        // division gate

        let mut cs = ConstraintSystem::<F>::init();

        let a = cs.new_variable(F::from(6));
        let b = cs.new_variable(F::from(3));
        let c = cs.division_gate(&a, &b);

        assert_eq!(cs.get_value(c), F::from(2));

        cs.assert_division(&a, &b, &c);

        assert!(cs.is_satisfied());

        let d = cs.new_variable(F::from(7));
        cs.assert_division(&a, &b, &d);
    }
}

fn test_boolean_helper<F: Field>() {
    let mut cs = ConstraintSystem::<F>::init();

    // assert one
    let a = cs.new_variable(F::from(1));
    cs.assert_one(&a);

    // assert zero
    let b = cs.new_variable(F::from(0));
    cs.assert_zero(&b);

    // assert binary
    cs.assert_binary(&a);
    cs.assert_binary(&b);

    assert!(cs.is_satisfied());
}

fn test_equality_helper<F: Field>() {
    let mut cs = ConstraintSystem::<F>::init();

    let a = cs.new_variable(F::from(2));
    let b = cs.new_variable(F::from(2));
    cs.assert_equal(&a, &b);

    assert!(cs.is_satisfied());

    let c = cs.new_variable(F::from(3));
    cs.assert_equal(&a, &c);

    assert!(!cs.is_satisfied());
}
