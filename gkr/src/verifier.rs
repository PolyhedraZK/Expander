use std::{
    io::{Cursor, Read},
    marker::PhantomData,
    vec,
};

use arith::Field;
use circuit::Circuit;
use gkr_engine::{
    ExpanderDualVarChallenge, ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, GKREngine,
    GKRScheme, MPIConfig, MPIEngine, PCSParams, Proof, StructuredReferenceString, Transcript,
};
use serdes::ExpSerde;
use sumcheck::{sumcheck_verify_gkr_layer, VerifierScratchPad};
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

// todo: FIXME
#[allow(clippy::type_complexity)]
pub fn gkr_verify<F: FieldEngine>(
    proving_time_mpi_size: usize,
    circuit: &Circuit<F>,
    public_input: &[F::SimdCircuitField],
    claimed_v: &F::ChallengeField,
    transcript: &mut impl Transcript<F::ChallengeField>,
    mut proof_reader: impl Read,
) -> (
    bool,
    ExpanderDualVarChallenge<F>,
    F::ChallengeField,
    Option<F::ChallengeField>,
) {
    let timer = Timer::new("gkr_verify", true);
    let mut sp = VerifierScratchPad::<F>::new(circuit, proving_time_mpi_size);

    let layer_num = circuit.layers.len();

    let mut challenge = ExpanderDualVarChallenge::sample_from_transcript(
        transcript,
        circuit.layers.last().unwrap().output_var_num,
        proving_time_mpi_size,
    );

    let mut alpha = None;
    let mut claimed_v0 = *claimed_v;
    let mut claimed_v1 = None;

    let mut verified = true;
    for i in (0..layer_num).rev() {
        let cur_verified = sumcheck_verify_gkr_layer(
            proving_time_mpi_size,
            &circuit.layers[i],
            public_input,
            &mut challenge,
            &mut claimed_v0,
            &mut claimed_v1,
            alpha,
            &mut proof_reader,
            transcript,
            &mut sp,
            i == layer_num - 1,
        );

        verified &= cur_verified;
        alpha = if challenge.rz_1.is_some() {
            Some(transcript.generate_challenge_field_element())
        } else {
            None
        };
    }
    timer.stop();
    let challenge = ExpanderDualVarChallenge::new(
        challenge.rz_0,
        challenge.rz_1,
        challenge.r_simd,
        challenge.r_mpi,
    );

    (verified, challenge, claimed_v0, claimed_v1)
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
        let proving_time_mpi_size = self.mpi_config.world_size();

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

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        #[cfg(feature = "grinding")]
        grind::<Cfg>(&mut transcript, &self.mpi_config);

        circuit.fill_rnd_coefs(&mut transcript);
        transcript_verifier_sync(&mut transcript, proving_time_mpi_size);

        let verified = match Cfg::SCHEME {
            GKRScheme::Vanilla => {
                let (mut verified, challenge, claimed_v0, claimed_v1) = gkr_verify(
                    proving_time_mpi_size,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                verified &= pcs_verified;
                log::info!("GKR verification: {}", verified);

                transcript_verifier_sync(&mut transcript, proving_time_mpi_size);

                let mut challenge_x = challenge.challenge_x();

                verified &= self.get_pcs_opening_from_proof_and_verify(
                    pcs_params,
                    pcs_verification_key,
                    &commitment,
                    &mut challenge_x,
                    &claimed_v0,
                    &mut transcript,
                    &mut cursor,
                );

                if challenge.rz_1.is_some() {
                    transcript_verifier_sync(&mut transcript, proving_time_mpi_size);

                    let mut challenge_y = challenge.challenge_y();
                    verified &= self.get_pcs_opening_from_proof_and_verify(
                        pcs_params,
                        pcs_verification_key,
                        &commitment,
                        &mut challenge_y,
                        &claimed_v1.unwrap(),
                        &mut transcript,
                        &mut cursor,
                    );
                }

                verified
            }
            GKRScheme::GkrSquare => {
                let (mut verified, mut challenge, claimed_v) = gkr_square_verify(
                    proving_time_mpi_size,
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
                    &mut challenge,
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
        open_at: &mut ExpanderSingleVarChallenge<Cfg::FieldConfig>,
        v: &<Cfg::FieldConfig as FieldEngine>::ChallengeField,
        transcript: &mut impl Transcript<<Cfg::FieldConfig as FieldEngine>::ChallengeField>,
        proof_reader: impl Read,
    ) -> bool {
        let opening = <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Opening::deserialize_from(
            proof_reader,
        )
        .unwrap();

        let minimum_vars_for_pcs: usize = pcs_params.num_vars();
        if open_at.rz.len() < minimum_vars_for_pcs {
            eprintln!(
				"{} over {} has minimum supported local vars {}, but challenge has vars {}, pad to {} vars in verifying.",
				Cfg::PCSConfig::NAME,
				<Cfg::FieldConfig as FieldEngine>::SimdCircuitField::NAME,
				minimum_vars_for_pcs,
				open_at.rz.len(),
				minimum_vars_for_pcs,
			);
            open_at.rz.resize(
                minimum_vars_for_pcs,
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
