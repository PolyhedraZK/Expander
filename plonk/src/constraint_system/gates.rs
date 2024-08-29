use arith::Field;
use ark_std::log2;
use halo2curves::ff::PrimeField;

use crate::{ConstraintSystem, FFTDomain, PublicKey, VAR_ONE};

use super::{GatesID, VariableIndex, VAR_ZERO};

// Gate implementations
impl<F: Field + PrimeField> ConstraintSystem<F> {
    /// constant gate
    #[inline]
    pub fn constant_gate(&mut self, c: &F) -> VariableIndex {
        let var_c = self.new_variable(*c);

        self.q_l.push(F::one());
        self.q_r.push(F::zero());
        self.q_o.push(F::zero());
        self.q_m.push(F::zero());
        self.q_c.push(-*c);

        self.a.push(var_c);
        self.b.push(VAR_ZERO);
        self.c.push(VAR_ZERO);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Constants);

        var_c
    }

    /// public input gate
    #[inline]
    pub fn public_input_gate(&mut self, pi: F) -> VariableIndex {
        // update pi list
        let row_index = self.q_l.get_nv();
        self.public_inputs_indices.push(row_index);

        let var_pi = self.new_variable(pi);

        self.q_l.push(-F::one());
        self.q_r.push(F::zero());
        self.q_o.push(F::zero());
        self.q_m.push(F::zero());
        self.q_c.push(F::zero());

        self.a.push(var_pi);
        self.b.push(VAR_ZERO);
        self.c.push(VAR_ZERO);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Constants);

        var_pi
    }

    /// Assert two variables are equal
    #[inline]
    pub fn assert_equal(&mut self, a: &VariableIndex, b: &VariableIndex) {
        self.q_l.push(F::one());
        self.q_r.push(-F::one());
        self.q_o.push(F::zero());
        self.q_m.push(F::zero());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(*b);
        self.c.push(VAR_ZERO);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Equal);
    }

    /// Assert the variable is zero
    #[inline]
    pub fn assert_zero(&mut self, a: &VariableIndex) {
        let a_val = self.get_value(*a);
        assert!(a_val == F::zero(), "a should be zero");

        self.q_l.push(F::one());
        self.q_r.push(F::zero());
        self.q_o.push(F::zero());
        self.q_m.push(F::zero());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(VAR_ZERO);
        self.c.push(VAR_ZERO);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Binary);
    }

    /// Assert the variable is one
    #[inline]
    pub fn assert_one(&mut self, a: &VariableIndex) {
        let a_val = self.get_value(*a);
        assert!(a_val == F::one(), "a should be one");

        self.q_l.push(F::one());
        self.q_r.push(F::zero());
        self.q_o.push(-F::one());
        self.q_m.push(F::zero());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(VAR_ZERO);
        self.c.push(VAR_ONE);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Constants);
    }

    /// Assert the variable is binary
    ///
    /// this is handled by constraint `a * (a - 1) = 0`
    #[inline]
    pub fn assert_binary(&mut self, a: &VariableIndex) {
        let a_val = self.get_value(*a);
        assert!(
            a_val == F::zero() || a_val == F::one(),
            "a should be binary"
        );

        self.q_l.push(-F::one());
        self.q_r.push(F::zero());
        self.q_o.push(F::zero());
        self.q_m.push(F::one());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(*a);
        self.c.push(VAR_ZERO);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Constants);
    }

    /// Assert the variable is not zero
    ///
    /// this is handled by adding a new variable `a_inv` and asserting `a * a_inv = 1`
    #[inline]
    pub fn assert_nonzero(&mut self, a: &VariableIndex) {
        let a_val = self.get_value(*a);
        assert!(a_val != F::zero(), "a should not be zero");
        let a_inv = a_val.inv().unwrap(); // safe unwrap
        let a_inv_var = self.new_variable(a_inv);

        self.q_l.push(F::zero());
        self.q_r.push(F::zero());
        self.q_o.push(-F::one());
        self.q_m.push(F::one());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(a_inv_var);
        self.c.push(VAR_ONE);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::NonZero);
    }

    /// addition gate: return the variable index of a + b
    #[inline]
    pub fn addition_gate(&mut self, a: &VariableIndex, b: &VariableIndex) -> VariableIndex {
        let a_val = self.get_value(*a);
        let b_val = self.get_value(*b);
        let c_val = a_val + b_val;
        let c = self.new_variable(c_val);

        self.assert_addition(a, b, &c);
        c
    }

    /// assert addition is correct: c = a + b
    #[inline]
    pub fn assert_addition(&mut self, a: &VariableIndex, b: &VariableIndex, c: &VariableIndex) {
        self.q_l.push(F::one());
        self.q_r.push(F::one());
        self.q_o.push(-F::one());
        self.q_m.push(F::zero());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(*b);
        self.c.push(*c);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Add);
    }

    /// subtraction gate: return the variable index of a - b
    #[inline]
    pub fn subtraction_gate(&mut self, a: &VariableIndex, b: &VariableIndex) -> VariableIndex {
        let a_val = self.get_value(*a);
        let b_val = self.get_value(*b);
        let c_val = a_val - b_val;
        let c = self.new_variable(c_val);

        self.assert_subtraction(a, b, &c);

        c
    }

    /// assert subtraction is correct: c = a - b
    #[inline]
    pub fn assert_subtraction(&mut self, a: &VariableIndex, b: &VariableIndex, c: &VariableIndex) {
        self.assert_addition(c, b, a)
    }

    /// multiplication gate: return the variable index of a * b
    #[inline]
    pub fn multiplication_gate(&mut self, a: &VariableIndex, b: &VariableIndex) -> VariableIndex {
        let a_val = self.get_value(*a);
        let b_val = self.get_value(*b);
        let c_val = a_val * b_val;
        let c = self.new_variable(c_val);

        self.assert_multiplication(a, b, &c);

        c
    }

    /// assert multiplication is correct: c = a * b
    #[inline]
    pub fn assert_multiplication(
        &mut self,
        a: &VariableIndex,
        b: &VariableIndex,
        c: &VariableIndex,
    ) {
        self.q_l.push(F::zero());
        self.q_r.push(F::zero());
        self.q_o.push(-F::one());
        self.q_m.push(F::one());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(*b);
        self.c.push(*c);

        #[cfg(feature = "print-gates")]
        self.gates.push(GatesID::Mul);
    }

    /// division gate: return the variable index of a / b
    #[inline]
    pub fn division_gate(&mut self, a: &VariableIndex, b: &VariableIndex) -> VariableIndex {
        self.assert_nonzero(b);
        let a_val = self.get_value(*a);
        let b_val = self.get_value(*b);
        let c_val = a_val * b_val.inv().unwrap(); // safe unwrap
        let c = self.new_variable(c_val);

        self.assert_division(a, b, &c);

        c
    }

    /// assert division is correct: c = a / b
    #[inline]
    pub fn assert_division(&mut self, a: &VariableIndex, b: &VariableIndex, c: &VariableIndex) {
        self.assert_multiplication(c, b, a)
    }

    /// selection gate: return the variable index of s * a + (1 - s) * b
    #[inline]
    pub fn selection_gate(
        &mut self,
        s: &VariableIndex,
        a: &VariableIndex,
        b: &VariableIndex,
    ) -> VariableIndex {
        let s_val = self.get_value(*s);
        let a_val = self.get_value(*a);
        let b_val = self.get_value(*b);
        let c_val = s_val * a_val + (F::one() - s_val) * b_val;
        let c = self.new_variable(c_val);

        self.assert_selection(s, a, b, &c);

        c
    }

    /// assert selection is correct: c = s * a + (1 - s) * b
    ///
    /// c = s * (a - b) + b
    ///
    /// statements:
    /// - s is binary
    /// - t1 = a - b
    /// - t2 = s * t1
    /// - c = t2 + b
    ///
    /// This requires 4 rows of constraints
    #[inline]
    pub fn assert_selection(
        &mut self,
        s: &VariableIndex,
        a: &VariableIndex,
        b: &VariableIndex,
        c: &VariableIndex,
    ) {
        // s is binary
        self.assert_binary(s);

        // t1 = a - b
        let t1 = self.subtraction_gate(a, b);

        // t2 = s * t1
        let t2 = self.multiplication_gate(s, &t1);

        // c = t2 + b
        self.assert_addition(&t2, b, c);
    }
}
