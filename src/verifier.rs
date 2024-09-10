use std::{io::Cursor, vec};

use arith::{ExtensionField, Field};
use ark_std::{end_timer, start_timer};

#[cfg(feature = "grinding")]
use crate::grind;

use crate::{
    eq_evals_at_primitive, Circuit, CircuitLayer, Config, FieldType, GKRConfig, Gate, Proof,
    RawCommitment, Transcript, _eq_vec,
};

#[inline]
fn degree_2_eval<C: GKRConfig>(
    p0: C::ChallengeField,
    p1: C::ChallengeField,
    p2: C::ChallengeField,
    x: C::ChallengeField,
) -> C::ChallengeField {
    if C::FIELD_TYPE == FieldType::GF2 {
        let c0 = &p0;
        let c2 = (p2 - p0 - p1.mul_by_x() + p0.mul_by_x())
            * (C::ChallengeField::X - C::ChallengeField::one())
                .mul_by_x()
                .inv()
                .unwrap();
        let c1 = p1 - p0 - c2;
        *c0 + (c2 * x + c1) * x
    } else {
        let c0 = &p0;
        let c2 = C::ChallengeField::INV_2 * (p2 - p1 - p1 + p0);
        let c1 = p1 - p0 - c2;
        *c0 + (c2 * x + c1) * x
    }
}

#[inline(always)]
fn lag_eval<F: Field + ExtensionField>(base: &[F], vals: &[F], x: &F) -> F {
    debug_assert_eq!(base.len(), vals.len());
    // trivial lag eval:
    let mut v = F::zero();
    for i in 0..base.len() {
        let mut numerator = F::one();
        let mut denominator = F::one();
        for j in 0..base.len() {
            if j == i {
                continue;
            }
            numerator *= *x - base[j];
            denominator *= base[i] - base[j];
        }
        v += numerator * denominator.inv().unwrap() * vals[i];
    }
    v
}

#[inline]
fn degree_3_eval<C: GKRConfig>(
    p0: C::ChallengeField,
    p1: C::ChallengeField,
    p2: C::ChallengeField,
    p3: C::ChallengeField,
    x: C::ChallengeField,
) -> C::ChallengeField {
    // TODO-OPTIMIZATION: precompute values and inverses
    if C::FIELD_TYPE == FieldType::GF2 {
        lag_eval(
            &[
                C::ChallengeField::zero(),
                C::ChallengeField::one(),
                C::ChallengeField::X,
                C::ChallengeField::X.mul_by_x(),
            ],
            &[p0, p1, p2, p3],
            &x,
        )
    } else {
        lag_eval(
            &[
                C::ChallengeField::zero(),
                C::ChallengeField::one(),
                C::ChallengeField::from(2),
                C::ChallengeField::from(3),
            ],
            &[p0, p1, p2, p3],
            &x,
        )
    }
}

// TODO: Remove redundant computation and split it into cst, add/uni and mul
#[allow(clippy::too_many_arguments)]
fn eval_sparse_circuit_connect_poly<C: GKRConfig, const INPUT_NUM: usize>(
    gates: &[Gate<C, INPUT_NUM>],
    rz0: &[C::ChallengeField],
    rz1: &[C::ChallengeField],
    r_simd: &[C::ChallengeField],
    alpha: C::ChallengeField,
    beta: C::ChallengeField,
    rx: &[C::ChallengeField],
    ry: &[C::ChallengeField],
    r_simd_xy: &[C::ChallengeField],
) -> C::ChallengeField {
    let mut eq_evals_at_rz0 = vec![C::ChallengeField::zero(); 1 << rz0.len()];
    let mut eq_evals_at_rz1 = vec![C::ChallengeField::zero(); 1 << rz1.len()];
    let mut eq_evals_at_r_simd = vec![C::ChallengeField::zero(); 1 << r_simd.len()];

    let mut eq_evals_at_rx = vec![C::ChallengeField::zero(); 1 << rx.len()];
    let mut eq_evals_at_ry = vec![C::ChallengeField::zero(); 1 << ry.len()];
    let mut eq_evals_at_r_simd_xy = vec![C::ChallengeField::zero(); 1 << r_simd_xy.len()];

    eq_evals_at_primitive(rz0, &alpha, &mut eq_evals_at_rz0);
    eq_evals_at_primitive(rz1, &beta, &mut eq_evals_at_rz1);
    eq_evals_at_primitive(r_simd, &C::ChallengeField::one(), &mut eq_evals_at_r_simd);

    eq_evals_at_primitive(rx, &C::ChallengeField::one(), &mut eq_evals_at_rx);
    eq_evals_at_primitive(ry, &C::ChallengeField::one(), &mut eq_evals_at_ry);
    eq_evals_at_primitive(
        r_simd_xy,
        &C::ChallengeField::one(),
        &mut eq_evals_at_r_simd_xy,
    );

    if INPUT_NUM == 0 {
        let mut v = C::ChallengeField::zero();

        for cst_gate in gates {
            v += C::challenge_mul_circuit_field(
                &(eq_evals_at_rz0[cst_gate.o_id] + eq_evals_at_rz1[cst_gate.o_id]),
                &cst_gate.coef,
            );
        }

        let simd_sum: C::ChallengeField = eq_evals_at_r_simd.iter().sum();
        v * simd_sum
    } else if INPUT_NUM == 1 {
        let mut v = C::ChallengeField::zero();
        for add_gate in gates {
            let tmp =
                C::challenge_mul_circuit_field(&eq_evals_at_rx[add_gate.i_ids[0]], &add_gate.coef);
            v += (eq_evals_at_rz0[add_gate.o_id] + eq_evals_at_rz1[add_gate.o_id]) * tmp;
        }
        v * _eq_vec(r_simd, r_simd_xy)
    } else if INPUT_NUM == 2 {
        let mut v = C::ChallengeField::zero();
        for mul_gate in gates {
            let tmp = eq_evals_at_rx[mul_gate.i_ids[0]]
                * C::challenge_mul_circuit_field(
                    &eq_evals_at_ry[mul_gate.i_ids[1]],
                    &mul_gate.coef,
                );
            v += (eq_evals_at_rz0[mul_gate.o_id] + eq_evals_at_rz1[mul_gate.o_id]) * tmp;
        }
        v * _eq_vec(r_simd, r_simd_xy)
    } else {
        unreachable!()
    }
}

#[inline(always)]
fn verify_sumcheck_step<C: GKRConfig>(
    proof: &mut Proof,
    degree: usize,
    transcript: &mut Transcript<C::FiatShamirHashType>,
    claimed_sum: &mut C::ChallengeField,
    randomness_vec: &mut Vec<C::ChallengeField>,
) -> bool {
    let mut ps = vec![];
    for i in 0..(degree + 1) {
        ps.push(proof.get_next_and_step());
        transcript.append_challenge_f::<C>(&ps[i]);
    }

    let r = transcript.challenge_f::<C>();
    randomness_vec.push(r);

    let verified = (ps[0] + ps[1]) == *claimed_sum;

    if degree == 2 {
        *claimed_sum = degree_2_eval::<C>(ps[0], ps[1], ps[2], r);
    } else if degree == 3 {
        *claimed_sum = degree_3_eval::<C>(ps[0], ps[1], ps[2], ps[3], r);
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
            alpha,
            beta,
            &[],
            &[],
            &[],
        );

    let mut rx = vec![];
    let mut ry = vec![];
    let mut r_simd_xy = vec![];
    let mut verified = true;

    for _i_var in 0..var_num {
        verified &= verify_sumcheck_step::<C>(proof, 2, transcript, &mut sum, &mut rx);
        // println!("x {} var, verified? {}", _i_var, verified);
    }

    for _i_var in 0..simd_var_num {
        verified &= verify_sumcheck_step::<C>(proof, 3, transcript, &mut sum, &mut r_simd_xy);
        // println!("{} simd var, verified? {}", _i_var, verified);
    }

    let vx_claim = proof.get_next_and_step::<C::ChallengeField>();
    sum -= vx_claim
        * eval_sparse_circuit_connect_poly(
            &layer.add,
            rz0,
            rz1,
            r_simd0,
            alpha,
            beta,
            &rx,
            &[],
            &r_simd_xy,
        );
    transcript.append_challenge_f::<C>(&vx_claim);

    for _i_var in 0..var_num {
        verified &= verify_sumcheck_step::<C>(proof, 2, transcript, &mut sum, &mut ry);
        // println!("y {} var, verified? {}", _i_var, verified);
    }

    let vy_claim = proof.get_next_and_step::<C::ChallengeField>();
    verified &= sum
        == vx_claim
            * vy_claim
            * eval_sparse_circuit_connect_poly(
                &layer.mul, rz0, rz1, r_simd0, alpha, beta, &rx, &ry, &r_simd_xy,
            );
    transcript.append_challenge_f::<C>(&vy_claim);
    (verified, rx, ry, r_simd_xy, vx_claim, vy_claim)
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
    C::ChallengeField,
    C::ChallengeField,
) {
    let timer = start_timer!(|| "gkr verify");
    let layer_num = circuit.layers.len();
    let mut rz0 = vec![];
    let mut rz1 = vec![];
    let mut r_simd = vec![];

    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.challenge_f::<C>());
        rz1.push(C::ChallengeField::zero());
    }

    for _ in 0..C::get_field_pack_size().trailing_zeros() {
        r_simd.push(transcript.challenge_f::<C>());
    }

    let mut alpha = C::ChallengeField::one();
    let mut beta = C::ChallengeField::zero();
    let mut claimed_v0 = *claimed_v;
    let mut claimed_v1 = C::ChallengeField::zero();

    let mut verified = true;
    for i in (0..layer_num).rev() {
        let cur_verified;
        (cur_verified, rz0, rz1, r_simd, claimed_v0, claimed_v1) = sumcheck_verify_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            &r_simd,
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
    (verified, rz0, rz1, r_simd, claimed_v0, claimed_v1)
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

        let (mut verified, rz0, rz1, r_simd, claimed_v0, claimed_v1) =
            gkr_verify(circuit, claimed_v, &mut transcript, &mut proof);

        log::info!("GKR verification: {}", verified);

        match self.config.polynomial_commitment_type {
            crate::PolynomialCommitmentType::Raw => {
                // for Raw, no need to load from proof
                log::trace!("rz0.size() = {}", rz0.len());
                log::trace!("Poly_vals.size() = {}", commitment.poly_vals.len());

                let v1 = commitment.verify(&rz0, &r_simd, claimed_v0);
                let v2 = commitment.verify(&rz1, &r_simd, claimed_v1);

                verified &= v1;
                verified &= v2;
            }
            _ => todo!(),
        }

        end_timer!(timer);

        verified
    }
}
