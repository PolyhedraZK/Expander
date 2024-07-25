use std::{io::Cursor, marker::PhantomData, vec};

use arith::{Field, FieldSerde, SimdField};
use ark_std::{end_timer, start_timer};

use crate::{
    eq_evals_at_primitive, grind, Circuit, CircuitLayer, Config, GKRConfig, Gate, Proof,
    RawCommitment, Transcript,
};

#[inline]
fn degree_2_eval<F: Field + SimdField>(p0: F, p1: F, p2: F, x: F::Scalar) -> F {
    let c0 = &p0;
    let c2 = F::INV_2 * (p2 - p1 - p1 + p0);
    let c1 = p1 - p0 - c2;
    *c0 + (c2.scale(&x) + c1).scale(&x)
}

fn eval_sparse_circuit_connect_poly<C: GKRConfig, const INPUT_NUM: usize>(
    gates: &[Gate<C, INPUT_NUM>],
    rz0: &[C::ChallengeField], //
    rz1: &[C::ChallengeField], //
    alpha: C::ChallengeField,
    beta: C::ChallengeField,
    ris: &[Vec<C::ChallengeField>],
) -> C::ChallengeField {
    let mut eq_evals_at_rz0 = vec![C::ChallengeField::zero(); 1 << rz0.len()];
    let mut eq_evals_at_rz1 = vec![C::ChallengeField::zero(); 1 << rz1.len()];

    eq_evals_at_primitive(rz0, &alpha, &mut eq_evals_at_rz0);
    eq_evals_at_primitive(rz1, &beta, &mut eq_evals_at_rz1);

    let mut eq_evals_at_ris = vec![vec![]; INPUT_NUM];
    for i in 0..INPUT_NUM {
        eq_evals_at_ris[i] = vec![C::ChallengeField::zero(); 1 << ris[i].len()];
        eq_evals_at_primitive(&ris[i], &C::ChallengeField::one(), &mut eq_evals_at_ris[i])
    }

    let mut v = C::ChallengeField::zero();
    for g in gates {
        let mut prod = eq_evals_at_rz0[g.o_id] + eq_evals_at_rz1[g.o_id];

        for (i, eq_evals_at_ri) in eq_evals_at_ris.iter().enumerate().take(INPUT_NUM) {
            prod *= eq_evals_at_ri[g.i_ids[i]];
        }
        v += C::challenge_mul_circuit(&prod, &g.coef);
    }
    v
}

// todo: FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn sumcheck_verify_gkr_layer<C: GKRConfig>(
    layer: &CircuitLayer<C>,
    rz0: &[C::ChallengeField],
    rz1: &[C::ChallengeField],
    claimed_v0: C::Field,
    claimed_v1: C::Field,
    alpha: C::ChallengeField,
    beta: C::ChallengeField,
    proof: &mut Proof,
    transcript: &mut Transcript,
    _config: &Config,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::Field,
    C::Field,
) {
    let var_num = layer.input_var_num;
    let mut sum = claimed_v0.scale(&alpha) + claimed_v1.scale(&beta)
        - C::Field::from(eval_sparse_circuit_connect_poly(
            &layer.const_,
            rz0,
            rz1,
            alpha,
            beta,
            &[],
        ));

    let mut rx = vec![];
    let mut ry = vec![];
    let mut vx_claim = C::Field::zero();
    let mut verified = true;
    for i_var in 0..var_num * 2 {
        let p0 = proof.get_next_and_step();
        let p1 = proof.get_next_and_step();
        let p2 = proof.get_next_and_step();
        transcript.append_f::<C>(p0);
        transcript.append_f::<C>(p1);
        transcript.append_f::<C>(p2);

        log::trace!("i_var={} p0 p1 p2: {:?} {:?} {:?}", i_var, p0, p1, p2);
        let r = transcript.challenge_f::<C>();

        if i_var < var_num {
            rx.push(r);
        } else {
            ry.push(r);
        }
        verified &= (p0 + p1) == sum;

        sum = degree_2_eval(p0, p1, p2, r);

        if i_var == var_num - 1 {
            vx_claim = proof.get_next_and_step();
            sum -= vx_claim.scale(&eval_sparse_circuit_connect_poly(
                &layer.add,
                rz0,
                rz1,
                alpha,
                beta,
                &[rx.clone()],
            ));
            transcript.append_f::<C>(vx_claim);
        }
    }
    let vy_claim: C::Field = proof.get_next_and_step();
    verified &= sum
        == vx_claim
            * vy_claim.scale(&eval_sparse_circuit_connect_poly(
                &layer.mul,
                rz0,
                rz1,
                alpha,
                beta,
                &[rx.clone(), ry.clone()],
            ));
    transcript.append_f::<C>(vy_claim);
    (verified, rx, ry, vx_claim, vy_claim)
}

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_verify<C: GKRConfig>(
    circuit: &Circuit<C>,
    claimed_v: &C::Field,
    transcript: &mut Transcript,
    proof: &mut Proof,
    config: &Config,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::Field,
    C::Field,
) {
    let timer = start_timer!(|| "gkr verify");
    let layer_num = circuit.layers.len();
    let mut rz0 = vec![];
    let mut rz1 = vec![];
    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.challenge_f::<C>());
        rz1.push(C::ChallengeField::zero());
    }
    let mut alpha = C::ChallengeField::one();
    let mut beta = C::ChallengeField::zero();
    let mut claimed_v0 = *claimed_v;
    let mut claimed_v1 = C::Field::zero();

    let mut verified = true;
    for i in (0..layer_num).rev() {
        let cur_verified;
        (cur_verified, rz0, rz1, claimed_v0, claimed_v1) = sumcheck_verify_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            claimed_v0,
            claimed_v1,
            alpha,
            beta,
            proof,
            transcript,
            config,
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
    (verified, rz0, rz1, claimed_v0, claimed_v1)
}

pub struct Verifier<C: GKRConfig> {
    config: Config,
    phantom: PhantomData<C>,
}

impl<C: GKRConfig> Verifier<C> {
    // todo: FIXME
    pub fn new(config: &Config) -> Self {
        Verifier {
            config: config.clone(),
            phantom: PhantomData,
        }
    }

    pub fn verify(&self, circuit: &Circuit<C>, claimed_v: &C::Field, proof: &Proof) -> bool {
        let timer = start_timer!(|| "verify");

        let poly_size = circuit.layers.first().unwrap().input_vals.evals.len();
        let mut cursor = Cursor::new(&proof.bytes);

        let commitment = RawCommitment::<C>::deserialize_from(&mut cursor, poly_size);

        let mut transcript = Transcript::new();
        transcript.append_u8_slice(&proof.bytes[..commitment.size()]);

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        grind::<C>(&mut transcript, &self.config);
        let mut proof = proof.clone(); // FIXME: consider separating pointers to make proof always immutable?
        proof.step(commitment.size() + 256 / 8);

        let (mut verified, rz0, rz1, claimed_v0, claimed_v1) = gkr_verify(
            circuit,
            claimed_v,
            &mut transcript,
            &mut proof,
            &self.config,
        );

        log::info!("GKR verification: {}", verified);

        match self.config.polynomial_commitment_type {
            crate::PolynomialCommitmentType::Raw => {
                // for Raw, no need to load from proof
                log::trace!("rz0.size() = {}", rz0.len());
                log::trace!("Poly_vals.size() = {}", commitment.poly_vals.len());

                let v1 = commitment.verify(&rz0, claimed_v0);
                let v2 = commitment.verify(&rz1, claimed_v1);

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
