use std::{
    io::{Cursor, Read},
    vec,
};

use arith::Field;
use circuit::Circuit;
use config::{Config, GKRConfig, GKRScheme};
use gkr_field_config::GKRFieldConfig;
use poly_commit::{ExpanderGKRChallenge, PCSForExpanderGKR, StructuredReferenceString};
use serdes::ExpSerde;
use transcript::{transcript_verifier_sync, Proof, Transcript};
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

impl<Cfg: GKRConfig> Verifier<Cfg> {
    pub fn new(config: &Config<Cfg>) -> Self {
        Verifier {
            config: config.clone(),
        }
    }

    pub fn verify(
        &self,
        circuit: &mut Circuit<Cfg::FieldConfig>,
        public_input: &[<Cfg::FieldConfig as GKRFieldConfig>::SimdCircuitField],
        claimed_v: &<Cfg::FieldConfig as GKRFieldConfig>::ChallengeField,
        pcs_params: &<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::Params,
        pcs_verification_key: &<<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::SRS as StructuredReferenceString>::VKey,
        proof: &Proof,
    ) -> bool {
        let timer = Timer::new("verify", true);
        let mut transcript = Cfg::Transcript::new();

        let mut cursor = Cursor::new(&proof.bytes);

        let commitment =
            <<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::Commitment as ExpSerde>::deserialize_from(
                &mut cursor,
            )
            .unwrap();
        let mut buffer = vec![];
        commitment.serialize_into(&mut buffer).unwrap();

        // this function will iteratively hash the commitment, and append the
        // this introduces a decent circuit depth for the FS transform.
        //
        // note that this function is almost identical to grind, except that grind uses a
        // fixed hasher, where as this function uses the transcript hasher
        transcript.append_commitment(&buffer);

        // TODO: Implement a trait containing the size function,
        // and use the following line to avoid unnecessary deserialization and serialization
        // transcript.append_u8_slice(&proof.bytes[..commitment.size()]);

        transcript_verifier_sync(&mut transcript, self.config.mpi_config.world_size());

        // ZZ: shall we use probabilistic grinding so the verifier can avoid this cost?
        // (and also be recursion friendly)
        #[cfg(feature = "grinding")]
        grind::<Cfg>(&mut transcript, &self.config);

        circuit.fill_rnd_coefs(&mut transcript);

        let gkr_verified;
        let (rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1);

        let mut verified = match self.config.gkr_scheme {
            GKRScheme::Vanilla => {
                (
                    gkr_verified,
                    rz0,
                    rz1,
                    r_simd,
                    r_mpi,
                    claimed_v0,
                    claimed_v1,
                ) = gkr_verify(
                    &self.config.mpi_config,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                gkr_verified
            }
            GKRScheme::GkrSquare => {
                (gkr_verified, rz0, r_simd, r_mpi, claimed_v0) = gkr_square_verify(
                    &self.config.mpi_config,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );
                rz1 = None;
                claimed_v1 = None;

                gkr_verified
            }
            GKRScheme::GKRParVerifier => {
                (
                    gkr_verified,
                    rz0,
                    rz1,
                    r_simd,
                    r_mpi,
                    claimed_v0,
                    claimed_v1,
                ) = gkr_par_verifier_verify(
                    &self.config.mpi_config,
                    circuit,
                    public_input,
                    claimed_v,
                    &mut transcript,
                    &mut cursor,
                );

                gkr_verified
            }
        };
        log::info!("GKR verification: {}", verified);

        transcript_verifier_sync(&mut transcript, self.config.mpi_config.world_size());

        verified &= self.get_pcs_opening_from_proof_and_verify(
            pcs_params,
            pcs_verification_key,
            &commitment,
            &mut ExpanderGKRChallenge {
                x: rz0,
                x_simd: r_simd.clone(),
                x_mpi: r_mpi.clone(),
            },
            &claimed_v0,
            &mut transcript,
            &mut cursor,
        );

        if let Some(rz1) = rz1 {
            transcript_verifier_sync(&mut transcript, self.config.mpi_config.world_size());
            verified &= self.get_pcs_opening_from_proof_and_verify(
                pcs_params,
                pcs_verification_key,
                &commitment,
                &mut ExpanderGKRChallenge {
                    x: rz1,
                    x_simd: r_simd,
                    x_mpi: r_mpi,
                },
                &claimed_v1.unwrap(),
                &mut transcript,
                &mut cursor,
            );
        }

        timer.stop();

        verified
    }
}

impl<Cfg: GKRConfig> Verifier<Cfg> {
    #[allow(clippy::too_many_arguments)]
    fn get_pcs_opening_from_proof_and_verify(
        &self,
        pcs_params: &<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::Params,
        pcs_verification_key: &<<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::SRS as StructuredReferenceString>::VKey,
        commitment: &<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::Commitment,
        open_at: &mut ExpanderGKRChallenge<Cfg::FieldConfig>,
        v: &<Cfg::FieldConfig as GKRFieldConfig>::ChallengeField,
        transcript: &mut Cfg::Transcript,
        proof_reader: impl Read,
    ) -> bool {
        let opening = <Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::Opening::deserialize_from(
            proof_reader,
        )
		.unwrap();

        if open_at.x.len() < Cfg::PCS::MINIMUM_NUM_VARS {
            eprintln!(
				"{} over {} has minimum supported local vars {}, but challenge has vars {}, pad to {} vars in verifying.",
				Cfg::PCS::NAME,
				<Cfg::FieldConfig as GKRFieldConfig>::SimdCircuitField::NAME,
				Cfg::PCS::MINIMUM_NUM_VARS,
				open_at.x.len(),
				Cfg::PCS::MINIMUM_NUM_VARS,
			);
            open_at.x.resize(
                Cfg::PCS::MINIMUM_NUM_VARS,
                <Cfg::FieldConfig as GKRFieldConfig>::ChallengeField::ZERO,
            )
        }

        transcript.lock_proof();
        let verified = Cfg::PCS::verify(
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
