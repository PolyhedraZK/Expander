mod gates;

mod selectors;
pub use selectors::Selector;

mod public_inputs;
pub use public_inputs::PublicInputsIndices;

mod variables;
pub use variables::*;

#[cfg(feature = "print-gates")]
mod gates_id;
#[cfg(feature = "print-gates")]
pub use gates_id::GatesID;

use arith::Field;
use ark_std::log2;
use halo2curves::ff::{PrimeField, WithSmallOrderMulGroup};

use crate::{CosetFFTDomain, FFTDomain};

/// Constraint system for the vanilla plonk protocol.
///
/// Vanilla plonk gate:
///
/// q_l * a + q_r * b + q_o * c + q_m * a * b + q_c + PI = 0
///
/// where
/// - `a`, `b`, `c` are the variables of the constraint system.
/// - `q_l`, `q_r`, `q_o`, `q_m` are the coefficients of the constraint system.
/// - `q_c` is the constant term of the constraint system.
/// - `PI` is the public input
///
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConstraintSystem<F: PrimeField> {
    /// selectors
    pub q_l: Selector<F>,
    pub q_r: Selector<F>,
    pub q_o: Selector<F>,
    pub q_m: Selector<F>,
    pub q_c: Selector<F>,

    /// those are the indexes of the witnesses
    pub a: VariableColumn,
    pub b: VariableColumn,
    pub c: VariableColumn,

    /// public inputs
    pub public_inputs_indices: PublicInputsIndices,

    /// the actual witnesses
    pub variables: Variables<F>,

    #[cfg(feature = "print-gates")]
    pub gates: Vec<GatesID>,

    /// fft domain
    pub eval_domain: Option<FFTDomain<F>>,

    /// coset domain
    pub coset_domain: Option<CosetFFTDomain<F>>,
}

impl<F: Field + PrimeField + WithSmallOrderMulGroup<3>> ConstraintSystem<F> {
    /// initialize a new constraint system with default constants
    #[inline]
    pub fn init() -> Self {
        let mut cs = ConstraintSystem::default();

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

            #[cfg(feature = "print-gates")]
            cs.gates.push(GatesID::Constants);
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

            #[cfg(feature = "print-gates")]
            cs.gates.push(GatesID::Constants);
        }
        cs
    }

    /// Finalize the constraint system and set the evaluation domains
    #[inline]
    pub fn finalize(&mut self) {
        let n = self.q_l.get_nv();
        assert_eq!(self.q_r.get_nv(), n);
        assert_eq!(self.q_o.get_nv(), n);
        assert_eq!(self.q_m.get_nv(), n);
        assert_eq!(self.q_c.get_nv(), n);
        assert_eq!(self.a.len(), n);
        assert_eq!(self.b.len(), n);
        assert_eq!(self.c.len(), n);

        let log_n = log2(n);
        let new_n = 1 << log_n;

        self.q_l.q.resize(new_n, F::zero());
        self.q_r.q.resize(new_n, F::zero());
        self.q_o.q.resize(new_n, F::zero());
        self.q_m.q.resize(new_n, F::zero());
        self.q_c.q.resize(new_n, F::zero());

        self.a.resize(new_n, VAR_ZERO);
        self.b.resize(new_n, VAR_ZERO);
        self.c.resize(new_n, VAR_ZERO);

        self.eval_domain = Some(FFTDomain::new(log_n as usize));
        self.coset_domain = Some(CosetFFTDomain::new(log_n as usize));
    }

    /// build the witness polynomials for a, b, and c
    #[inline]
    pub(crate) fn build_witness_polynomials(&self) -> [Vec<F>; 3] {
        let n = self.q_l.get_nv();
        let mut a = vec![F::zero(); n];
        let mut b = vec![F::zero(); n];
        let mut c = vec![F::zero(); n];

        for i in 0..n {
            a[i] = self.variables.witnesses[self.a[i]];
            b[i] = self.variables.witnesses[self.b[i]];
            c[i] = self.variables.witnesses[self.c[i]];
        }

        [a, b, c]
    }

    /// create a new variable
    #[inline]
    pub fn new_variable(&mut self, f: F) -> VariableIndex {
        self.variables.new_variable(f)
    }

    /// get the field element of a variable
    #[inline]
    pub fn get_value(&self, index: VariableIndex) -> F {
        self.variables.witnesses[index]
    }

    /// check the constraint system is satisfied
    #[inline]
    pub fn is_satisfied(&self, public_inputs: &[F]) -> bool {
        if public_inputs.len() != self.public_inputs_indices.row_index.len() {
            println!("public inputs length not match");
            return false;
        }

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

        let pi = self
            .public_inputs_indices
            .build_pi_poly(public_inputs, length);

        println!("public inputs: {:?}", self.public_inputs_indices);
        println!("public inputs: {:?}", pi);
        println!("public inputs: {:?}", public_inputs);

        for index in 0..length {
            let a = self.get_value(self.a[index]);
            let b = self.get_value(self.b[index]);
            let c = self.get_value(self.c[index]);

            let q_l = self.q_l.q[index];
            let q_r = self.q_r.q[index];
            let q_o = self.q_o.q[index];
            let q_m = self.q_m.q[index];
            let q_c = self.q_c.q[index];

            let pi_i = pi[index];

            if a * q_l + b * q_r + c * q_o + a * b * q_m + q_c + pi_i != F::zero() {
                #[cfg(not(feature = "print-gates"))]
                println!("cs failed at row {}", index,);
                #[cfg(feature = "print-gates")]
                println!(
                    "cs failed at row {} with gate: {:?}",
                    index, self.gates[index]
                );

                println!("a:   {:?}", a);
                println!("b:   {:?}", b);
                println!("c:   {:?}", c);
                println!("pi:  {:?}", pi_i);
                println!("q_l: {:?}", q_l);
                println!("q_r: {:?}", q_r);
                println!("q_o: {:?}", q_o);
                println!("q_m: {:?}", q_m);
                println!("q_c: {:?}", q_c);
                println!();
                return false;
            }
        }

        // todo: permutation checks

        true
    }
}
