mod sumcheck_verifier_helper;

pub use sumcheck_verifier_helper::*;

use std::{io::Cursor, vec};

use arith::{ExtensionField, Field};
use ark_std::{end_timer, start_timer};

#[cfg(feature = "grinding")]
use crate::grind;

use crate::{
    eq_evals_at_primitive, Circuit, CircuitLayer, Config, FieldType, GKRConfig, Gate, Proof,
    RawCommitment, Transcript, _eq_vec,
};

#[inline(always)]
fn verify_sumcheck_step<C: GKRConfig>(
    proof: &mut Proof,
    degree: usize,
    transcript: &mut Transcript<C::FiatShamirHashType>,
    claimed_sum: &mut C::ChallengeField,
    randomness_vec: &mut Vec<C::ChallengeField>,
    sp: &VerifierScratchPad<C>,
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
        *claimed_sum = GKRVerifierHelper::degree_2_eval(&ps, r, sp);
    } else if degree == 3 {
        *claimed_sum = GKRVerifierHelper::degree_3_eval(&ps, r, sp);
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
    r_simd: &Vec<C::ChallengeField>,
    claimed_v0: C::ChallengeField,
    claimed_v1: C::ChallengeField,
    alpha: C::ChallengeField,
    beta: C::ChallengeField,
    proof: &mut Proof,
    transcript: &mut Transcript<C::FiatShamirHashType>,
    sp: &mut VerifierScratchPad<C>,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::ChallengeField,
    C::ChallengeField,
) {
    GKRVerifierHelper::prepare_layer(layer, &alpha, &beta, rz0, rz1, r_simd, sp);

    let var_num = layer.input_var_num;
    let simd_var_num = C::get_field_pack_size().trailing_zeros() as usize;
    let mut sum =
        claimed_v0 * alpha + claimed_v1 * beta - GKRVerifierHelper::eval_cst(&layer.const_, sp);

    let mut rx = vec![];
    let mut ry = vec![];
    let mut r_simd_xy = vec![];
    let mut verified = true;

    for _i_var in 0..var_num {
        verified &= verify_sumcheck_step::<C>(proof, 2, transcript, &mut sum, &mut rx, sp);
        // println!("x {} var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_rx(&rx, sp);

    for _i_var in 0..simd_var_num {
        verified &= verify_sumcheck_step::<C>(proof, 3, transcript, &mut sum, &mut r_simd_xy, sp);
        // println!("{} simd var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_r_simd_xy(&r_simd_xy, sp);

    let vx_claim = proof.get_next_and_step::<C::ChallengeField>();
    sum -= vx_claim * GKRVerifierHelper::eval_add(&layer.add, sp);
    transcript.append_challenge_f::<C>(&vx_claim);

    for _i_var in 0..var_num {
        verified &= verify_sumcheck_step::<C>(proof, 2, transcript, &mut sum, &mut ry, sp);
        // println!("y {} var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_ry(&ry, sp);

    let vy_claim = proof.get_next_and_step::<C::ChallengeField>();
    verified &= sum == vx_claim * vy_claim * GKRVerifierHelper::eval_mul(&layer.mul, sp);
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
    let mut sp = VerifierScratchPad::<C>::new(circuit);

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
            &mut sp,
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
