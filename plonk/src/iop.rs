use halo2curves::ff::PrimeField;

use crate::ConstraintSystem;

pub struct PlonkIOP;

impl PlonkIOP {
    /// Generate a vanishing polynomial for the IOP.
    ///
    /// h(x) = q_l(x) * a(x) + q_r(x) * b(x) + q_o(x) * c(x) + q_m(x) * a(x) * b(x) + q_c(x)
    ///
    /// this polynomial should be of degree 2n if all constraints are satisfied
    pub fn generate_vanishing_polynomial<F: PrimeField>(cs: &ConstraintSystem<F>) {
        // let q_l_x = cs.eval_domain.ifft(&cs.q_l);
        // let q_r_x = cs.eval_domain.ifft(&cs.q_r);
        // let q_o_x = cs.eval_domain.ifft(&cs.q_o);
        // let q_m_x = cs.eval_domain.ifft(&cs.q_m);
        // let q_c_x = cs.eval_domain.ifft(&cs.q_c);
    }
}
