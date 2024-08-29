use halo2curves::ff::{PrimeField, WithSmallOrderMulGroup};

use crate::ConstraintSystem;

/// The public key of the protocol.
///
/// Consists of the coset evaluations of the selector polynomials.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PublicKey<F: PrimeField> {
    pub q_l_coset: Vec<F>,
    pub q_r_coset: Vec<F>,
    pub q_o_coset: Vec<F>,
    pub q_m_coset: Vec<F>,
    pub q_c_coset: Vec<F>,
}

impl<F: PrimeField + WithSmallOrderMulGroup<3>> PublicKey<F> {
    pub fn extract_public_key(cs: &ConstraintSystem<F>) -> Self {
        let domain = match &cs.eval_domain {
            Some(domain) => domain,
            None => panic!("cs is not finalized yet"),
        };
        let coset_domain = match &cs.coset_domain {
            Some(domain) => domain,
            None => panic!("cs is not finalized yet"),
        };

        let q_l_x = domain.ifft(&cs.q_l.q);
        let q_l_coset = coset_domain.coset_fft(&q_l_x);

        let q_r_x = domain.ifft(&cs.q_r.q);
        let q_r_coset = coset_domain.coset_fft(&q_r_x);

        let q_o_x = domain.ifft(&cs.q_o.q);
        let q_o_coset = coset_domain.coset_fft(&q_o_x);

        let q_m_x = domain.ifft(&cs.q_m.q);
        let q_m_coset = coset_domain.coset_fft(&q_m_x);

        let q_c_x = domain.ifft(&cs.q_c.q);
        let q_c_coset = coset_domain.coset_fft(&q_c_x);

        Self {
            q_l_coset,
            q_r_coset,
            q_o_coset,
            q_m_coset,
            q_c_coset,
        }
    }
}
