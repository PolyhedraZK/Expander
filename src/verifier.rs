use std::{io::Cursor, vec};

use arith::{ExtensionField, Field};
use ark_std::{end_timer, start_timer};

#[cfg(feature = "grinding")]
use crate::grind;

use crate::{
    eq_evals_at_primitive, Circuit, CircuitLayer, Config, FieldType, GKRConfig, Gate, Proof,
    RawCommitment, Transcript, _eq_vec, _eq_vec_3,
};

#[inline]
fn degree_2_eval<F: Field + ExtensionField>(p0: F, p1: F, p2: F, x: F) -> F {
    let c0 = &p0;
    let c2 = F::INV_2 * (p2 - p1 - p1 + p0);
    let c1 = p1 - p0 - c2;
    *c0 + (c2 * x + c1) * x
}

#[inline]
fn gf2_sp_eval<F: Field + ExtensionField>(p0: F, p1: F, p2: F, x: F) -> F {
    let c0 = &p0;
    let c2 =
        (p2 - p0 - p1.mul_by_x() + p0.mul_by_x()) * (F::X - F::one()).mul_by_x().inv().unwrap();
    let c1 = p1 - p0 - c2;
    *c0 + (c2 * x + c1) * x
}

// TODO: Remove redundant computation and split it into cst, add/uni and mul
#[allow(clippy::too_many_arguments)]
fn eval_sparse_circuit_connect_poly<C: GKRConfig, const INPUT_NUM: usize>(
    gates: &[Gate<C, INPUT_NUM>],
    rz0: &[C::ChallengeField],
    rz1: &[C::ChallengeField],
    r_simd0: &[C::ChallengeField],
    r_simd1: &[C::ChallengeField],
    alpha: C::ChallengeField,
    beta: C::ChallengeField,
    rx: &[C::ChallengeField],
    ry: &[C::ChallengeField],
    r_simdx: &[C::ChallengeField],
    r_simdy: &[C::ChallengeField],
) -> C::ChallengeField {
    let mut eq_evals_at_rz0 = vec![C::ChallengeField::zero(); 1 << rz0.len()];
    let mut eq_evals_at_rz1 = vec![C::ChallengeField::zero(); 1 << rz1.len()];
    let mut eq_evals_at_r_simd0 = vec![C::ChallengeField::zero(); 1 << r_simd0.len()];
    let mut eq_evals_at_r_simd1 = vec![C::ChallengeField::zero(); 1 << r_simd1.len()];

    let mut eq_evals_at_rx = vec![C::ChallengeField::zero(); 1 << rx.len()];
    let mut eq_evals_at_ry = vec![C::ChallengeField::zero(); 1 << ry.len()];
    let mut eq_evals_at_r_simdx = vec![C::ChallengeField::zero(); 1 << r_simdx.len()];
    let mut eq_evals_at_r_simdy = vec![C::ChallengeField::zero(); 1 << r_simdy.len()];

    eq_evals_at_primitive(rz0, &alpha, &mut eq_evals_at_rz0);
    eq_evals_at_primitive(rz1, &beta, &mut eq_evals_at_rz1);
    eq_evals_at_primitive(r_simd0, &C::ChallengeField::one(), &mut eq_evals_at_r_simd0);
    eq_evals_at_primitive(r_simd1, &C::ChallengeField::one(), &mut eq_evals_at_r_simd1);

    eq_evals_at_primitive(rx, &C::ChallengeField::one(), &mut eq_evals_at_rx);
    eq_evals_at_primitive(ry, &C::ChallengeField::one(), &mut eq_evals_at_ry);
    eq_evals_at_primitive(r_simdx, &C::ChallengeField::one(), &mut eq_evals_at_r_simdx);
    eq_evals_at_primitive(r_simdy, &C::ChallengeField::one(), &mut eq_evals_at_r_simdy);

    if INPUT_NUM == 0 {
        let mut v0 = C::ChallengeField::zero();
        let mut v1 = C::ChallengeField::zero();

        for cst_gate in gates {
            v0 += C::challenge_mul_circuit_field(&eq_evals_at_rz0[cst_gate.o_id], &cst_gate.coef);
            v1 += C::challenge_mul_circuit_field(&eq_evals_at_rz1[cst_gate.o_id], &cst_gate.coef);
        }

        let simd_sum0: C::ChallengeField = eq_evals_at_r_simd0.iter().sum();
        let simd_sum1: C::ChallengeField = eq_evals_at_r_simd1.iter().sum();
        v0 * simd_sum0 + v1 * simd_sum1
    } else if INPUT_NUM == 1 {
        let mut v0 = C::ChallengeField::zero();
        let mut v1 = C::ChallengeField::zero();
        for add_gate in gates {
            let tmp =
                C::challenge_mul_circuit_field(&eq_evals_at_rx[add_gate.i_ids[0]], &add_gate.coef);
            v0 += eq_evals_at_rz0[add_gate.o_id] * tmp;
            v1 += eq_evals_at_rz1[add_gate.o_id] * tmp;
        }
        v0 * _eq_vec(r_simd0, r_simdx) + v1 * _eq_vec(r_simd1, r_simdx)
    } else if INPUT_NUM == 2 {
        let mut v0 = C::ChallengeField::zero();
        let mut v1 = C::ChallengeField::zero();
        for mul_gate in gates {
            let tmp = eq_evals_at_rx[mul_gate.i_ids[0]]
                * C::challenge_mul_circuit_field(
                    &eq_evals_at_ry[mul_gate.i_ids[1]],
                    &mul_gate.coef,
                );
            v0 += eq_evals_at_rz0[mul_gate.o_id] * tmp;
            v1 += eq_evals_at_rz1[mul_gate.o_id] * tmp;
        }
        v0 * _eq_vec_3(r_simd0, r_simdx, r_simdy) + v1 * _eq_vec_3(r_simd1, r_simdx, r_simdy)
    } else {
        unreachable!()
    }
}

#[inline(always)]
fn verify_sumcheck_step<C: GKRConfig>(
    proof: &mut Proof,
    transcript: &mut Transcript<C::FiatShamirHashType>,
    claimed_sum: &mut C::ChallengeField,
    randomness_vec: &mut Vec<C::ChallengeField>,
) -> bool {
    let p0 = proof.get_next_and_step();
    let p1 = proof.get_next_and_step();
    let p2 = proof.get_next_and_step();
    transcript.append_challenge_f::<C>(&p0);
    transcript.append_challenge_f::<C>(&p1);
    transcript.append_challenge_f::<C>(&p2);

    let r = transcript.challenge_f::<C>();
    randomness_vec.push(r);

    let verified = (p0 + p1) == *claimed_sum;

    if C::FIELD_TYPE == FieldType::GF2 {
        *claimed_sum = gf2_sp_eval(p0, p1, p2, r);
    } else {
        *claimed_sum = degree_2_eval(p0, p1, p2, r);
    }

    verified
}

// todo: FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn sumcheck_verify_gkr_layer<C: GKRConfig>(
    layer: &CircuitLayer<C>,
    rz0: &[C::ChallengeField],
    rz1: &[C::ChallengeField],
    r_simd0: &[C::ChallengeField],
    r_simd1: &[C::ChallengeField],
    claimed_v0: C::ChallengeField,
    claimed_v1: C::ChallengeField,
    alpha: C::ChallengeField,
    beta: C::ChallengeField,
    proof: &mut Proof,
    transcript: &mut Transcript<C::FiatShamirHashType>,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::ChallengeField,
    C::ChallengeField,
) {
    let var_num = layer.input_var_num;
    let simd_var_num = C::get_field_pack_size().trailing_zeros() as usize;
    let mut sum = claimed_v0 * alpha + claimed_v1 * beta
        - eval_sparse_circuit_connect_poly(
            &layer.const_,
            rz0,
            rz1,
            r_simd0,
            r_simd1,
            alpha,
            beta,
            &[],
            &[],
            &[],
            &[],
        );

    let mut rx = vec![];
    let mut ry = vec![];
    let mut r_simdx = vec![];
    let mut r_simdy = vec![];
    let mut verified = true;

    for _i_var in 0..var_num {
        verified &= verify_sumcheck_step::<C>(proof, transcript, &mut sum, &mut rx);
        // println!("x {} var, verified? {}", _i_var, verified);
    }

    for _i_var in 0..simd_var_num {
        verified &= verify_sumcheck_step::<C>(proof, transcript, &mut sum, &mut r_simdx);
        // println!("x {} simd var, verified? {}", _i_var, verified);
    }

    let vx_claim = proof.get_next_and_step::<C::ChallengeField>();
    sum -= vx_claim
        * eval_sparse_circuit_connect_poly(
            &layer.add,
            rz0,
            rz1,
            r_simd0,
            r_simd1,
            alpha,
            beta,
            &rx,
            &[],
            &r_simdx,
            &[],
        );
    transcript.append_challenge_f::<C>(&vx_claim);

    for _i_var in 0..var_num {
        verified &= verify_sumcheck_step::<C>(proof, transcript, &mut sum, &mut ry);
        // println!("y {} var, verified? {}", _i_var, verified);
    }

    for _i_var in 0..simd_var_num {
        verified &= verify_sumcheck_step::<C>(proof, transcript, &mut sum, &mut r_simdy);
        // println!("y {} simd var, verified? {}", _i_var, verified);
    }

    let vy_claim = proof.get_next_and_step::<C::ChallengeField>();
    verified &= sum
        == vx_claim
            * vy_claim
            * eval_sparse_circuit_connect_poly(
                &layer.mul, rz0, rz1, r_simd0, r_simd1, alpha, beta, &rx, &ry, &r_simdx, &r_simdy,
            );
    transcript.append_challenge_f::<C>(&vy_claim);
    (verified, rx, ry, r_simdx, r_simdy, vx_claim, vy_claim)
}

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_verify<C: GKRConfig>(
    circuit: &Circuit<C>,
    claimed_v: &C::ChallengeField,
    transcript: &mut Transcript<C::FiatShamirHashType>,
    proof: &mut Proof,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::ChallengeField,
    C::ChallengeField,
) {
    let timer = start_timer!(|| "gkr verify");
    let layer_num = circuit.layers.len();
    let mut rz0 = vec![];
    let mut rz1 = vec![];
    let mut r_simd0 = vec![];
    let mut r_simd1 = vec![];

    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.challenge_f::<C>());
        rz1.push(C::ChallengeField::zero());
    }

    for _ in 0..C::get_field_pack_size().trailing_zeros() {
        r_simd0.push(transcript.challenge_f::<C>());
        r_simd1.push(C::ChallengeField::zero());
    }

    let mut alpha = C::ChallengeField::one();
    let mut beta = C::ChallengeField::zero();
    let mut claimed_v0 = *claimed_v;
    let mut claimed_v1 = C::ChallengeField::zero();

    let mut verified = true;
    for i in (0..layer_num).rev() {
        let cur_verified;
        (
            cur_verified,
            rz0,
            rz1,
            r_simd0,
            r_simd1,
            claimed_v0,
            claimed_v1,
        ) = sumcheck_verify_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            &r_simd0,
            &r_simd1,
            claimed_v0,
            claimed_v1,
            alpha,
            beta,
            proof,
            transcript,
        );
        verified &= cur_verified;
        alpha = transcript.challenge_f::<C>();
        beta = transcript.challenge_f::<C>();
        log::trace!(
            "Layer {} verified with alpha={:?} and beta={:?}, claimed_v0={:?}, claimed_v1={:?}",
            i,
            alpha,
            beta,
            claimed_v0,
            claimed_v1
        );
    }
    end_timer!(timer);
    (verified, rz0, rz1, r_simd0, r_simd1, claimed_v0, claimed_v1)
}

pub struct Verifier<C: GKRConfig> {
    config: Config<C>,
}

impl<C: GKRConfig> Default for Verifier<C> {
    fn default() -> Self {
        Self {
            config: Config::<C>::default(),
        }
    }
}

impl<C: GKRConfig> Verifier<C> {
    pub fn new(config: &Config<C>) -> Self {
        Verifier {
            config: config.clone(),
        }
    }

    pub fn verify(
        &self,
        circuit: &mut Circuit<C>,
        claimed_v: &C::ChallengeField,
        proof: &Proof,
    ) -> bool {
        let timer = start_timer!(|| "verify");

        let poly_size = circuit.layers.first().unwrap().input_vals.len();
        let mut cursor = Cursor::new(&proof.bytes);

        let commitment = RawCommitment::<C>::deserialize_from(&mut cursor, poly_size);

        let mut transcript = Transcript::new();
        transcript.append_u8_slice(&proof.bytes[..commitment.size()]);

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        #[cfg(feature = "grinding")]
        grind::<C>(&mut transcript, &self.config);

        circuit.fill_rnd_coefs(&mut transcript);

        let mut proof = proof.clone(); // FIXME: consider separating pointers to make proof always immutable?

        #[cfg(feature = "grinding")]
        proof.step(commitment.size() + 32);
        #[cfg(not(feature = "grinding"))]
        proof.step(commitment.size());

        let (mut verified, rz0, rz1, r_simd0, r_simd1, claimed_v0, claimed_v1) =
            gkr_verify(circuit, claimed_v, &mut transcript, &mut proof);

        log::info!("GKR verification: {}", verified);

        match self.config.polynomial_commitment_type {
            crate::PolynomialCommitmentType::Raw => {
                // for Raw, no need to load from proof
                log::trace!("rz0.size() = {}", rz0.len());
                log::trace!("Poly_vals.size() = {}", commitment.poly_vals.len());

                let v1 = commitment.verify(&rz0, &r_simd0, claimed_v0);
                let v2 = commitment.verify(&rz1, &r_simd1, claimed_v1);

                println!("Debug: v1 verified? {}", v1);
                println!("Debug: v2 verified? {}", v2);

                log::debug!("first commitment verification: {}", v1);
                log::debug!("second commitment verification: {}", v2);

                verified &= v1;
                verified &= v2;
            }
            _ => todo!(),
        }

        end_timer!(timer);

        verified
    }
}
