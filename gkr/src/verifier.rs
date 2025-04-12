use std::{
    io::{Cursor, Read},
    marker::PhantomData,
    vec,
};

use arith::Field;
use circuit::Circuit;
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, GKREngine, GKRScheme, MPIConfig,
    MPIEngine, PCSParams, Proof, StructuredReferenceString, Transcript,
};
use serdes::ExpSerde;
use transcript::transcript_verifier_sync;
use utils::timer::Timer;

#[cfg(feature = "grinding")]
use crate::grind;

mod common;
pub use common::*;

mod gkr_vanilla;
pub use gkr_vanilla::gkr_verify;

mod gkr_square;
pub use gkr_square::gkr_square_verify;

mod gkr_par_verifier;
pub use gkr_par_verifier::gkr_par_verifier_verify;

#[derive(Default)]
pub struct Verifier<Cfg: GKREngine> {
    pub mpi_config: MPIConfig,
    phantom: PhantomData<Cfg>,
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

        // this function will iteratively hash the commitment, and append the
        // final hash output to the transcript.
        // this introduces a decent circuit depth for the FS transform.
        //
        // note that this function is almost identical to grind, except that grind uses a
        // fixed hasher, where as this function uses the transcript hasher
        transcript.append_commitment(&buffer);

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        #[cfg(feature = "grinding")]
        grind::<Cfg>(&mut transcript, &self.mpi_config);

        circuit.fill_rnd_coefs(&mut transcript);
        transcript_verifier_sync(&mut transcript, proving_time_mpi_size);

        let (mut verified, mut challenge_x, challenge_y, claim_x, claim_y) = match Cfg::SCHEME {
            GKRScheme::Vanilla => {
                let (gkr_verified, challenge, claim_x, claim_y) = gkr_verify(
                    proving_time_mpi_size,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                (
                    gkr_verified,
                    challenge.challenge_x(),
                    challenge.challenge_y(),
                    claim_x,
                    claim_y,
                )
            }
            GKRScheme::GkrSquare => {
                let (gkr_verified, challenge_x, claim_x) = gkr_square_verify(
                    proving_time_mpi_size,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                (gkr_verified, challenge_x, None, claim_x, None)
            }
            GKRScheme::GKRParVerifier => {
                let (gkr_verified, challenge, claim_x, claim_y) = gkr_par_verifier_verify(
                    proving_time_mpi_size,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                (
                    gkr_verified,
                    challenge.challenge_x(),
                    challenge.challenge_y(),
                    claim_x,
                    claim_y,
                )
            }
        };
        log::info!("GKR verification: {}", verified);

        transcript_verifier_sync(&mut transcript, proving_time_mpi_size);

        verified &= self.get_pcs_opening_from_proof_and_verify(
            pcs_params,
            pcs_verification_key,
            &commitment,
            &mut challenge_x,
            &claim_x,
            &mut transcript,
            &mut cursor,
        );

        if let Some(mut challenge_y) = challenge_y {
            transcript_verifier_sync(&mut transcript, proving_time_mpi_size);
            verified &= self.get_pcs_opening_from_proof_and_verify(
                pcs_params,
                pcs_verification_key,
                &commitment,
                &mut challenge_y,
                &claim_y.unwrap(),
                &mut transcript,
                &mut cursor,
            );
        }

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
        transcript: &mut impl Transcript,
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
