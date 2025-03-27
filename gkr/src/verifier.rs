use std::{
    io::{Cursor, Read},
    marker::PhantomData,
    vec,
};

use arith::Field;
use circuit::{Circuit, CircuitLayer};
use gkr_engine::{
    ExpanderChallenge, ExpanderPCS, FieldEngine, GKREngine, GKRScheme, MPIConfig, MPIEngine, Proof,
    StructuredReferenceString, Transcript,
};
use serdes::ExpSerde;
use sumcheck::{
    GKRVerifierHelper, VerifierScratchPad, SUMCHECK_GKR_DEGREE, SUMCHECK_GKR_SIMD_MPI_DEGREE,
    SUMCHECK_GKR_SQUARE_DEGREE,
};
use transcript::transcript_verifier_sync;
use utils::timer::Timer;

#[cfg(feature = "grinding")]
use crate::grind;

mod gkr_square;
pub use gkr_square::gkr_square_verify;

#[derive(Default)]
pub struct Verifier<Cfg: GKREngine> {
    pub mpi_config: MPIConfig,
    phantom: PhantomData<Cfg>,
}

#[inline(always)]
fn verify_sumcheck_step<F: FieldEngine>(
    mut proof_reader: impl Read,
    degree: usize,
    transcript: &mut impl Transcript<F::ChallengeField>,
    claimed_sum: &mut F::ChallengeField,
    randomness_vec: &mut Vec<F::ChallengeField>,
    sp: &VerifierScratchPad<F>,
) -> bool {
    let mut ps = vec![];
    for i in 0..(degree + 1) {
        ps.push(F::ChallengeField::deserialize_from(&mut proof_reader).unwrap());
        transcript.append_field_element(&ps[i]);
    }

    let r = transcript.generate_challenge_field_element();
    randomness_vec.push(r);

    let verified = (ps[0] + ps[1]) == *claimed_sum;

    // This assumes SUMCHECK_GKR_DEGREE == 2, SUMCHECK_GKR_SIMD_MPI_DEGREE == 3,
    // SUMCHECK_GKR_SQUARE_DEGREE == 6
    if degree == SUMCHECK_GKR_DEGREE {
        *claimed_sum = GKRVerifierHelper::degree_2_eval(&ps, r, sp);
    } else if degree == SUMCHECK_GKR_SIMD_MPI_DEGREE {
        *claimed_sum = GKRVerifierHelper::degree_3_eval(&ps, r, sp);
    } else if degree == SUMCHECK_GKR_SQUARE_DEGREE {
        *claimed_sum = GKRVerifierHelper::degree_6_eval(&ps, r, sp);
    } else {
        panic!("unsupported degree");
    }

    verified
}

// todo: FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
#[allow(clippy::unnecessary_unwrap)]
fn sumcheck_verify_gkr_layer<F: FieldEngine>(
    mpi_config: &MPIConfig,
    layer: &CircuitLayer<F>,
    public_input: &[F::SimdCircuitField],
    rz0: &[F::ChallengeField],
    rz1: &Option<Vec<F::ChallengeField>>,
    r_simd: &Vec<F::ChallengeField>,
    r_mpi: &Vec<F::ChallengeField>,
    claimed_v0: F::ChallengeField,
    claimed_v1: Option<F::ChallengeField>,
    alpha: Option<F::ChallengeField>,
    mut proof_reader: impl Read,
    transcript: &mut impl Transcript<F::ChallengeField>,
    sp: &mut VerifierScratchPad<F>,
    is_output_layer: bool,
) -> (
    bool,
    Vec<F::ChallengeField>,
    Option<Vec<F::ChallengeField>>,
    Vec<F::ChallengeField>,
    Vec<F::ChallengeField>,
    F::ChallengeField,
    Option<F::ChallengeField>,
) {
    assert_eq!(rz1.is_none(), claimed_v1.is_none());
    assert_eq!(rz1.is_none(), alpha.is_none());

    GKRVerifierHelper::prepare_layer(layer, &alpha, rz0, rz1, r_simd, r_mpi, sp, is_output_layer);

    let var_num = layer.input_var_num;
    let simd_var_num = F::get_field_pack_size().trailing_zeros() as usize;
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
        verified &= verify_sumcheck_step::<F>(
            &mut proof_reader,
            SUMCHECK_GKR_DEGREE,
            transcript,
            &mut sum,
            &mut rx,
            sp,
        );
        // println!("x {} var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_rx(&rx, sp);

    for _i_var in 0..simd_var_num {
        verified &= verify_sumcheck_step::<F>(
            &mut proof_reader,
            SUMCHECK_GKR_SIMD_MPI_DEGREE,
            transcript,
            &mut sum,
            &mut r_simd_xy,
            sp,
        );
        // println!("{} simd var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_r_simd_xy(&r_simd_xy, sp);

    for _i_var in 0..mpi_config.world_size().trailing_zeros() {
        verified &= verify_sumcheck_step::<F>(
            &mut proof_reader,
            SUMCHECK_GKR_SIMD_MPI_DEGREE,
            transcript,
            &mut sum,
            &mut r_mpi_xy,
            sp,
        );
        // println!("{} mpi var, verified? {}", _i_var, verified);
    }
    GKRVerifierHelper::set_r_mpi_xy(&r_mpi_xy, sp);

    let vx_claim = F::ChallengeField::deserialize_from(&mut proof_reader).unwrap();

    sum -= vx_claim * GKRVerifierHelper::eval_add(&layer.add, sp);
    transcript.append_field_element(&vx_claim);

    let vy_claim = if !layer.structure_info.skip_sumcheck_phase_two {
        ry = Some(vec![]);
        for _i_var in 0..var_num {
            verified &= verify_sumcheck_step::<F>(
                &mut proof_reader,
                SUMCHECK_GKR_DEGREE,
                transcript,
                &mut sum,
                ry.as_mut().unwrap(),
                sp,
            );
            // println!("y {} var, verified? {}", _i_var, verified);
        }
        GKRVerifierHelper::set_ry(ry.as_ref().unwrap(), sp);

        let vy_claim = F::ChallengeField::deserialize_from(&mut proof_reader).unwrap();
        transcript.append_field_element(&vy_claim);
        verified &= sum == vx_claim * vy_claim * GKRVerifierHelper::eval_mul(&layer.mul, sp);
        Some(vy_claim)
    } else {
        verified &= sum == F::ChallengeField::ZERO;
        None
    };
    (verified, rx, ry, r_simd_xy, r_mpi_xy, vx_claim, vy_claim)
}

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_verify<F: FieldEngine>(
    mpi_config: &MPIConfig,
    circuit: &Circuit<F>,
    public_input: &[F::SimdCircuitField],
    claimed_v: &F::ChallengeField,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mut proof_reader: impl Read,
) -> (
    bool,
    Vec<F::ChallengeField>,
    Option<Vec<F::ChallengeField>>,
    Vec<F::ChallengeField>,
    Vec<F::ChallengeField>,
    F::ChallengeField,
    Option<F::ChallengeField>,
) {
    let timer = Timer::new("gkr_verify", true);
    let mut sp = VerifierScratchPad::<F>::new(circuit, mpi_config.world_size());

    let layer_num = circuit.layers.len();
    let mut rz0 = vec![];
    let mut rz1 = None;
    let mut r_simd = vec![];
    let mut r_mpi = vec![];

    for _ in 0..circuit.layers.last().unwrap().output_var_num {
        rz0.push(transcript.generate_challenge_field_element());
    }

    for _ in 0..F::get_field_pack_size().trailing_zeros() {
        r_simd.push(transcript.generate_challenge_field_element());
    }

    for _ in 0..mpi_config.world_size().trailing_zeros() {
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
            mpi_config,
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
    timer.stop();
    (verified, rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1)
}

impl<Cfg: GKREngine> Verifier<Cfg> {
    pub fn new(mpi_config: MPIConfig) -> Self {
        Self {
            mpi_config,
            phantom: PhantomData,
        }
    }

    pub fn verify(
        &self,
        circuit: &mut Circuit<Cfg::FieldConfig>,
        public_input: &[<Cfg::FieldConfig as FieldEngine>::SimdCircuitField],
        claimed_v: &<Cfg::FieldConfig as FieldEngine>::ChallengeField,
        pcs_params: &<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Params,
        pcs_verification_key: &<<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::SRS as StructuredReferenceString>::VKey,
        proof: &Proof,
    ) -> bool {
        let timer = Timer::new("verify", true);
        let mut transcript = Cfg::TranscriptConfig::new();

        let mut cursor = Cursor::new(&proof.bytes);

        let commitment =
            <<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Commitment as ExpSerde>::deserialize_from(
                &mut cursor,
            )
            .unwrap();
        let mut buffer = vec![];
        commitment.serialize_into(&mut buffer).unwrap();

        // this function will iteratively hash the commitment, and append the final hash output
        // to the transcript. this introduces a decent circuit depth for the FS transform.
        //
        // note that this function is almost identical to grind, except that grind uses a
        // fixed hasher, where as this function uses the transcript hasher
        let pcs_verified = transcript.append_commitment_and_check_digest(&buffer, &mut cursor);
        log::info!("pcs verification: {}", pcs_verified);

        // TODO: Implement a trait containing the size function,
        // and use the following line to avoid unnecessary deserialization and serialization
        // transcript.append_u8_slice(&proof.bytes[..commitment.size()]);

        transcript_verifier_sync(&mut transcript, self.mpi_config.world_size());

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        #[cfg(feature = "grinding")]
        grind::<Cfg>(&mut transcript, &self.mpi_config);

        circuit.fill_rnd_coefs(&mut transcript);

        let verified = match Cfg::SCHEME {
            GKRScheme::Vanilla => {
                let (mut verified, rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1) = gkr_verify(
                    &self.mpi_config,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                verified &= pcs_verified;
                log::info!("GKR verification: {}", verified);

                transcript_verifier_sync(&mut transcript, self.mpi_config.world_size());

                verified &= self.get_pcs_opening_from_proof_and_verify(
                    pcs_params,
                    pcs_verification_key,
                    &commitment,
                    &mut ExpanderChallenge {
                        x: rz0,
                        x_simd: r_simd.clone(),
                        x_mpi: r_mpi.clone(),
                    },
                    &claimed_v0,
                    &mut transcript,
                    &mut cursor,
                );

                if let Some(rz1) = rz1 {
                    transcript_verifier_sync(&mut transcript, self.mpi_config.world_size());
                    verified &= self.get_pcs_opening_from_proof_and_verify(
                        pcs_params,
                        pcs_verification_key,
                        &commitment,
                        &mut ExpanderChallenge {
                            x: rz1,
                            x_simd: r_simd,
                            x_mpi: r_mpi,
                        },
                        &claimed_v1.unwrap(),
                        &mut transcript,
                        &mut cursor,
                    );
                }

                verified
            }
            GKRScheme::GkrSquare => {
                let (mut verified, rz, r_simd, r_mpi, claimed_v) = gkr_square_verify(
                    &self.mpi_config,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                log::info!("GKR verification: {}", verified);

                verified &= self.get_pcs_opening_from_proof_and_verify(
                    pcs_params,
                    pcs_verification_key,
                    &commitment,
                    &mut ExpanderChallenge {
                        x: rz,
                        x_simd: r_simd.clone(),
                        x_mpi: r_mpi.clone(),
                    },
                    &claimed_v,
                    &mut transcript,
                    &mut cursor,
                );
                verified
            }
        };

        timer.stop();

        verified
    }
}

impl<Cfg: GKREngine> Verifier<Cfg> {
    #[allow(clippy::too_many_arguments)]
    fn get_pcs_opening_from_proof_and_verify(
        &self,
        pcs_params: &<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Params,
        pcs_verification_key: &<<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::SRS as StructuredReferenceString>::VKey,
        commitment: &<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Commitment,
        open_at: &mut ExpanderChallenge<Cfg::FieldConfig>,
        v: &<Cfg::FieldConfig as FieldEngine>::ChallengeField,
        transcript: &mut impl Transcript<<Cfg::FieldConfig as FieldEngine>::ChallengeField>,
        proof_reader: impl Read,
    ) -> bool {
        let opening = <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Opening::deserialize_from(
            proof_reader,
        )
        .unwrap();

        if open_at.x.len() < <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::MINIMUM_NUM_VARS {
            eprintln!(
				"{} over {} has minimum supported local vars {}, but challenge has vars {}, pad to {} vars in verifying.",
				Cfg::PCSConfig::NAME,
				<Cfg::FieldConfig as FieldEngine>::SimdCircuitField::NAME,
				Cfg::PCSConfig::MINIMUM_NUM_VARS,
				open_at.x.len(),
				Cfg::PCSConfig::MINIMUM_NUM_VARS,
			);
            open_at.x.resize(
                <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::MINIMUM_NUM_VARS,
                <Cfg::FieldConfig as FieldEngine>::ChallengeField::ZERO,
            )
        }

        transcript.lock_proof();
        let verified = Cfg::PCSConfig::verify(
            pcs_params,
            pcs_verification_key,
            commitment,
            open_at,
            *v,
            transcript,
            &opening,
        );
        transcript.unlock_proof();

        let mut buffer = vec![];
        opening.serialize_into(&mut buffer).unwrap(); // TODO: error propagation
        transcript.append_u8_slice(&buffer);

        verified
    }
}
