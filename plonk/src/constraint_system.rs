use arith::Field;

use crate::{
    selectors::Selector,
    variable::{VariableColumn, VariableIndex, VariableZero, Variables},
};

/// Constraint system for the vanilla plonk protocol.
///
/// Vanilla plonk gate:
/// 
/// q_l * a + q_r * b + q_o * c + q_m * a * b + q_c = 0
/// 
/// where
/// - `a`, `b`, `c` are the variables of the constraint system.
/// - `q_l`, `q_r`, `q_o`, `q_m` are the coefficients of the constraint system.
/// - `q_c` is the constant term of the constraint system.
///
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConstraintSystem<F> {
    pub q_l: Selector<F>,
    pub q_r: Selector<F>,
    pub q_o: Selector<F>,
    pub q_m: Selector<F>,
    pub q_c: Selector<F>,

    /// those are the indexes of the witnesses
    pub a: VariableColumn,
    pub b: VariableColumn,
    pub c: VariableColumn,

    /// the actual witnesses
    pub variables: Variables<F>,
}

impl<F: Field> ConstraintSystem<F> {
    /// Create a new, empty constraint system.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// initialize a new constraint system with default constants
    #[inline]
    pub fn init() -> Self {
        let mut cs = ConstraintSystem::new();

        let zero_var = cs.variables.new_variable(F::zero());
        let one_var = cs.variables.new_variable(F::one());

        // assert the first witness is 0
        {
            cs.q_l.push(F::one());
            cs.q_r.push(F::zero());
            cs.q_o.push(F::zero());
            cs.q_m.push(F::zero());
            cs.q_c.push(F::zero());

            cs.a.push(zero_var);
            cs.b.push(zero_var);
            cs.c.push(zero_var);
        }
        // assert the second witness is 1

        {
            cs.q_l.push(F::one());
            cs.q_r.push(F::zero());
            cs.q_o.push(F::zero());
            cs.q_m.push(F::zero());
            cs.q_c.push(-F::one());

            cs.a.push(one_var);
            cs.b.push(zero_var);
            cs.c.push(zero_var);
        }
        cs
    }

    #[inline]
    pub fn new_variable(&mut self, f: F) -> VariableIndex {
        self.variables.new_variable(f)
    }

    #[inline]
    pub fn get_value(&self, index: VariableIndex) -> F {
        self.variables.witnesses[index]
    }

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
        self.b.push(VariableZero);
        self.c.push(VariableZero);

        var_c
    }

    /// addition gate
    #[inline]
    pub fn addition_gate(&mut self, a: &VariableIndex, b: &VariableIndex) -> VariableIndex {
        let a_val = self.get_value(*a);
        let b_val = self.get_value(*b);
        let c_val = a_val + b_val;
        let c = self.new_variable(c_val);

        self.q_l.push(F::one());
        self.q_r.push(F::one());
        self.q_o.push(-F::one());
        self.q_m.push(F::zero());
        self.q_c.push(F::zero());

        self.a.push(*a);
        self.b.push(*b);
        self.c.push(c);

        c
    }

    #[inline]
    pub fn check_cs(&self) -> bool {
        let length = self.q_l.get_nv();

        if self.q_r.get_nv() != length {
            return false;
        }
        if self.q_o.get_nv() != length {
            return false;
        }
        if self.q_m.get_nv() != length {
            return false;
        }
        if self.q_c.get_nv() != length {
            return false;
        }

        for index in 0..length {
            let a = self.get_value(self.a[index]);
            let b = self.get_value(self.b[index]);
            let c = self.get_value(self.c[index]);

            let q_l = self.q_l.q[index];
            let q_r = self.q_r.q[index];
            let q_o = self.q_o.q[index];
            let q_m = self.q_m.q[index];
            let q_c = self.q_c.q[index];

            if a * q_l + b * q_r + c * q_o + a * b * q_m + q_c != F::zero() {
                println!("cs failed at row {}", index);
                println!("a: {:?}", a);
                println!("b: {:?}", b);
                println!("c: {:?}", c);
                println!("q_l: {:?}", q_l);
                println!("q_r: {:?}", q_r);
                println!("q_o: {:?}", q_o);
                println!("q_m: {:?}", q_m);
                println!("q_c: {:?}", q_c);
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {

    use arith::M31;

    use super::*;

    #[test]
    fn test_constraint_system() {
        let mut cs = ConstraintSystem::<M31>::init();

        let a = cs.new_variable(M31::from(3));
        let b = cs.new_variable(M31::from(4));

        let c = cs.addition_gate(&a, &b);
        let d = cs.addition_gate(&b, &c);

        assert_eq!(cs.get_value(c), M31::from(7));
        assert_eq!(cs.get_value(d), M31::from(11));

        assert_eq!(cs.check_cs(), true);
    }
}
