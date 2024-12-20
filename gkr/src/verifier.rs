use std::{
    io::{Cursor, Read},
    vec,
};

use arith::{Field, FieldSerde};
use ark_std::{end_timer, start_timer};
use circuit::{Circuit, CircuitLayer};
use config::{Config, FiatShamirHashType, GKRConfig, PolynomialCommitmentType};
use sumcheck::{GKRVerifierHelper, VerifierScratchPad};
use transcript::{
    BytesHashTranscript, FieldHashTranscript, Keccak256hasher, MIMCHasher, Proof, SHA256hasher,
    Transcript,
};

#[cfg(feature = "grinding")]
use crate::grind;
use crate::RawCommitment;

#[inline(always)]
fn verify_sumcheck_step<C: GKRConfig, T: Transcript<C::ChallengeField>>(
    mut proof_reader: impl Read,
    degree: usize,
    transcript: &mut T,
    claimed_sum: &mut C::ChallengeField,
    randomness_vec: &mut Vec<C::ChallengeField>,
    sp: &VerifierScratchPad<C>,
) -> bool {
    let mut ps = vec![];
    for i in 0..(degree + 1) {
        ps.push(C::ChallengeField::deserialize_from(&mut proof_reader).unwrap());
        transcript.append_field_element(&ps[i]);
    }

    let r = transcript.generate_challenge_field_element();
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
#[allow(clippy::unnecessary_unwrap)]
fn sumcheck_verify_gkr_layer<C: GKRConfig, T: Transcript<C::ChallengeField>>(
    config: &Config<C>,
    layer: &CircuitLayer<C>,
    public_input: &[C::SimdCircuitField],
    rz0: &[C::ChallengeField],
    rz1: &Option<Vec<C::ChallengeField>>,
    r_simd: &Vec<C::ChallengeField>,
    r_mpi: &Vec<C::ChallengeField>,
    claimed_v0: C::ChallengeField,
    claimed_v1: Option<C::ChallengeField>,
    alpha: Option<C::ChallengeField>,
    mut proof_reader: impl Read,
    transcript: &mut T,
    sp: &mut VerifierScratchPad<C>,
    is_output_layer: bool,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Option<Vec<C::ChallengeField>>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::ChallengeField,
    Option<C::ChallengeField>,
) {
    assert_eq!(rz1.is_none(), claimed_v1.is_none());
    assert_eq!(rz1.is_none(), alpha.is_none());

    GKRVerifierHelper::prepare_layer(layer, &alpha, rz0, rz1, r_simd, r_mpi, sp, is_output_layer);

    let var_num = layer.input_var_num;
    let simd_var_num = C::get_field_pack_size().trailing_zeros() as usize;
    let mut sum = claimed_v0;
    if claimed_v1.is_some() && alpha.is_some() {
        sum += claimed_v1.unwrap() * alpha.unwrap();
    }

    sum -= GKRVerifierHelper::eval_cst(&layer.const_, public_input, sp);

    let mut rx = vec![];
    let mut ry = None;
    let mut r_simd_xy = vec![];
    let mut r_mpi_xy = vec![];
    let mut verified = true;

    for _i_var in 0..var_num {
        verified &=
            verify_sumcheck_step::<C, T>(&mut proof_reader, 2, transcript, &mut sum, &mut rx, sp);
        // println!("x {} var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_rx(&rx, sp);

    for _i_var in 0..simd_var_num {
        verified &= verify_sumcheck_step::<C, T>(
            &mut proof_reader,
            3,
            transcript,
            &mut sum,
            &mut r_simd_xy,
            sp,
        );
        // println!("{} simd var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_r_simd_xy(&r_simd_xy, sp);

    for _i_var in 0..config.mpi_config.world_size().trailing_zeros() {
        verified &= verify_sumcheck_step::<C, T>(
            &mut proof_reader,
            3,
            transcript,
            &mut sum,
            &mut r_mpi_xy,
            sp,
        );
        // println!("{} mpi var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_r_mpi_xy(&r_mpi_xy, sp);

    let vx_claim = C::ChallengeField::deserialize_from(&mut proof_reader).unwrap();

    sum -= vx_claim * GKRVerifierHelper::eval_add(&layer.add, sp);
    transcript.append_field_element(&vx_claim);

    let vy_claim = if !layer.structure_info.max_degree_one {
        ry = Some(vec![]);
        for _i_var in 0..var_num {
            verified &= verify_sumcheck_step::<C, T>(
                &mut proof_reader,
                2,
                transcript,
                &mut sum,
                ry.as_mut().unwrap(),
                sp,
            );
            // println!("y {} var, verified? {}", _i_var, verified);
        }
        GKRVerifierHelper::set_ry(ry.as_ref().unwrap(), sp);

        let vy_claim = C::ChallengeField::deserialize_from(&mut proof_reader).unwrap();
        transcript.append_field_element(&vy_claim);
        verified &= sum == vx_claim * vy_claim * GKRVerifierHelper::eval_mul(&layer.mul, sp);
        Some(vy_claim)
    } else {
        verified &= sum == C::ChallengeField::ZERO;
        None
    };

    (verified, rx, ry, r_simd_xy, r_mpi_xy, vx_claim, vy_claim)
}

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_verify<C: GKRConfig, T: Transcript<C::ChallengeField>>(
    config: &Config<C>,
    circuit: &Circuit<C>,
    public_input: &[C::SimdCircuitField],
    claimed_v: &C::ChallengeField,
    transcript: &mut T,
    mut proof_reader: impl Read,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Option<Vec<C::ChallengeField>>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::ChallengeField,
    Option<C::ChallengeField>,
) {
    let timer = start_timer!(|| "gkr verify");
    let mut sp = VerifierScratchPad::<C>::new(config, circuit);

    let layer_num = circuit.layers.len();
    let mut rz0 = vec![];
    let mut rz1 = None;
    let mut r_simd = vec![];
    let mut r_mpi = vec![];

    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.generate_challenge_field_element());
    }

    for _ in 0..C::get_field_pack_size().trailing_zeros() {
        r_simd.push(transcript.generate_challenge_field_element());
    }

    for _ in 0..config.mpi_config.world_size().trailing_zeros() {
        r_mpi.push(transcript.generate_challenge_field_element());
    }

    let mut alpha = None;
    let mut claimed_v0 = *claimed_v;
    let mut claimed_v1 = None;

    let mut verified = true;
    for i in (0..layer_num).rev() {
        let cur_verified;
        (
            cur_verified,
            rz0,
            rz1,
            r_simd,
            r_mpi,
            claimed_v0,
            claimed_v1,
        ) = sumcheck_verify_gkr_layer(
            config,
            &circuit.layers[i],
            public_input,
            &rz0,
            &rz1,
            &r_simd,
            &r_mpi,
            claimed_v0,
            claimed_v1,
            alpha,
            &mut proof_reader,
            transcript,
            &mut sp,
            i == layer_num - 1,
        );
        verified &= cur_verified;
        alpha = if rz1.is_some() {
            Some(transcript.generate_challenge_field_element())
        } else {
            None
        };
    }
    end_timer!(timer);
    (verified, rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1)
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

    fn verify_internal<T: Transcript<C::ChallengeField>>(
        &self,
        circuit: &mut Circuit<C>,
        public_input: &[C::SimdCircuitField],
        claimed_v: &C::ChallengeField,
        proof: &Proof,
        transcript: &mut T,
    ) -> bool {
        let timer = start_timer!(|| "verify");

        let poly_size =
            circuit.layers.first().unwrap().input_vals.len() * self.config.mpi_config.world_size();
        let mut cursor = Cursor::new(&proof.bytes);

        let commitment = RawCommitment::<C>::deserialize_from(&mut cursor, poly_size);
        transcript.append_u8_slice(&proof.bytes[..commitment.size()]);

        if self.config.mpi_config.world_size() > 1 {
            let _ = transcript.hash_and_return_state(); // Trigger an additional hash
        }

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        #[cfg(feature = "grinding")]
        grind::<C, T>(transcript, &self.config);

        circuit.fill_rnd_coefs(transcript);

        let (mut verified, rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1) = gkr_verify(
            &self.config,
            circuit,
            public_input,
            claimed_v,
            transcript,
            &mut cursor,
        );

        log::info!("GKR verification: {}", verified);

        match self.config.polynomial_commitment_type {
            PolynomialCommitmentType::Raw => {
                // for Raw, no need to load from proof
                log::trace!("rz0.size() = {}", rz0.len());
                log::trace!("Poly_vals.size() = {}", commitment.poly_vals.len());

                let v1 = commitment.mpi_verify(&rz0, &r_simd, &r_mpi, claimed_v0);
                verified &= v1;

                if rz1.is_some() {
                    let v2 = commitment.mpi_verify(
                        rz1.as_ref().unwrap(),
                        &r_simd,
                        &r_mpi,
                        claimed_v1.unwrap(),
                    );
                    verified &= v2;
                }
            }
            _ => todo!(),
        }

        end_timer!(timer);

        verified
    }

    pub fn verify(
        &self,
        circuit: &mut Circuit<C>,
        public_input: &[C::SimdCircuitField],
        claimed_v: &C::ChallengeField,
        proof: &Proof,
    ) -> bool {
        match C::FIAT_SHAMIR_HASH {
            FiatShamirHashType::Keccak256 => {
                let mut transcript =
                    BytesHashTranscript::<C::ChallengeField, Keccak256hasher>::new();
                self.verify_internal(circuit, public_input, claimed_v, proof, &mut transcript)
            }
            FiatShamirHashType::SHA256 => {
                let mut transcript = BytesHashTranscript::<C::ChallengeField, SHA256hasher>::new();
                self.verify_internal(circuit, public_input, claimed_v, proof, &mut transcript)
            }
            FiatShamirHashType::MIMC5 => {
                let mut transcript =
                    FieldHashTranscript::<C::ChallengeField, MIMCHasher<C::ChallengeField>>::new();
                self.verify_internal(circuit, public_input, claimed_v, proof, &mut transcript)
            }
            _ => unreachable!(),
        }
    }
}
