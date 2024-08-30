use arith::{Field, UnivariatePolynomial};
use halo2curves::ff::{PrimeField, WithSmallOrderMulGroup};

use crate::{ConstraintSystem, LOG_EXT_DEGREE};

pub struct PlonkIOP;

impl PlonkIOP {
    /// Generate a zero polynomial for the IOP.
    ///
    /// h(x) = [ q_l(x) * a(x) + q_r(x) * b(x) + q_o(x) * c(x) + q_m(x) * a(x) * b(x) + q_c(x) ] / (x^n - 1)
    ///
    /// this polynomial should be of degree 2n if all constraints are satisfied
    pub fn generate_zero_polynomial<F: Field + PrimeField + WithSmallOrderMulGroup<3>>(
        cs: &ConstraintSystem<F>,
        pi_poly: &[F],
    ) -> Vec<F> {
        let n = cs.q_l.q.len();

        let eval_domain = match &cs.eval_domain {
            Some(domain) => domain,
            None => panic!("eval domain is not set: suspect cs not finalized"),
        };

        let coset_domain = match &cs.coset_domain {
            Some(domain) => domain,
            None => panic!("coset domain is not set: suspect cs not finalized"),
        };

        let mut hx_ext = vec![F::zero(); n << LOG_EXT_DEGREE];

        let [a, b, c] = cs.build_witness_polynomials();
        let pi_eval = cs.public_inputs_indices.build_pi_poly(pi_poly, n);

        // FIXME: drop the data after used to save memory
        let q_l_x = eval_domain.ifft(&cs.q_l.q);
        let q_l_x_ext = coset_domain.coset_fft(&q_l_x);

        let q_r_x = eval_domain.ifft(&cs.q_r.q);
        let q_r_x_ext = coset_domain.coset_fft(&q_r_x);

        let q_o_x = eval_domain.ifft(&cs.q_o.q);
        let q_o_x_ext = coset_domain.coset_fft(&q_o_x);

        let q_m_x = eval_domain.ifft(&cs.q_m.q);
        let q_m_x_ext = coset_domain.coset_fft(&q_m_x);

        let q_c_x = eval_domain.ifft(&cs.q_c.q);
        let q_c_x_ext = coset_domain.coset_fft(&q_c_x);

        let a_x = eval_domain.ifft(&a);
        let a_x_ext = coset_domain.coset_fft(&a_x);

        let b_x = eval_domain.ifft(&b);
        let b_x_ext = coset_domain.coset_fft(&b_x);

        let c_x = eval_domain.ifft(&c);
        let c_x_ext = coset_domain.coset_fft(&c_x);

        let pi_x = eval_domain.ifft(&pi_eval);
        let pi_x_ext = coset_domain.coset_fft(&pi_x);

        for i in 0..n << LOG_EXT_DEGREE {
            hx_ext[i] = q_l_x_ext[i] * a_x_ext[i]
                + q_r_x_ext[i] * b_x_ext[i]
                + q_o_x_ext[i] * c_x_ext[i]
                + q_m_x_ext[i] * a_x_ext[i] * b_x_ext[i]
                + q_c_x_ext[i]
                + pi_x_ext[i];
        }

        coset_domain.divide_by_vanishing_poly(&mut hx_ext);

        let hx = coset_domain.coset_ifft(&hx_ext);

        {
            // check hx is computed correctly via Schwartz-Zippel lemma
            // for debugging we use a fixed challenge
            let challenge = F::from(100u64);

            let q_l_x_eval = UnivariatePolynomial::new(q_l_x).evaluate(&challenge);
            let q_r_x_eval = UnivariatePolynomial::new(q_r_x).evaluate(&challenge);
            let q_o_x_eval = UnivariatePolynomial::new(q_o_x).evaluate(&challenge);
            let q_m_x_eval = UnivariatePolynomial::new(q_m_x).evaluate(&challenge);
            let q_c_x_eval = UnivariatePolynomial::new(q_c_x).evaluate(&challenge);

            let a_x_eval = UnivariatePolynomial::new(a_x).evaluate(&challenge);
            let b_x_eval = UnivariatePolynomial::new(b_x).evaluate(&challenge);
            let c_x_eval = UnivariatePolynomial::new(c_x).evaluate(&challenge);

            let pi_x_eval = UnivariatePolynomial::new(pi_x).evaluate(&challenge);

            let hx_eval = UnivariatePolynomial::new(hx.clone()).evaluate(&challenge);
            let tx_eval = challenge.pow_vartime([n as u64]) - F::one();

            assert_eq!(
                hx_eval * tx_eval,
                q_l_x_eval * a_x_eval
                    + q_r_x_eval * b_x_eval
                    + q_o_x_eval * c_x_eval
                    + q_m_x_eval * a_x_eval * b_x_eval
                    + q_c_x_eval
                    + pi_x_eval
            );
        }

        hx
    }

    /// compute the grand product polynomial
    /// \prod_i (
    ///     \frac{
    ///        (a_i + beta x^i + gamma)(b_i + beta x^i + gamma)(c_i + beta x^i + gamma)
    ///     }{
    ///        (a_i + beta sigma_a(x^i) + gamma)(b_i + beta sigma_b(x^i) + gamma)(c_i + beta sigma_c(x^i) + gamma)
    ///     }
    /// )
    /// where sigma_a, sigma_b, sigma_c are the permutation polynomials for a, b, c
    /// and beta, gamma are random challenges
    ///
    fn compute_grand_product<F: Field + PrimeField + WithSmallOrderMulGroup<3>>(
        cs: &ConstraintSystem<F>,
        beta: F,
        gamma: F,
    ) {
        // TODO
    }
}
