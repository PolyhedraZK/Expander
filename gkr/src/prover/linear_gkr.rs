//! This module implements the whole GKR prover, including the IOP and PCS.

use ark_std::{end_timer, start_timer};
use circuit::Circuit;
use config::{Config, FiatShamirHashType, GKRConfig, GKRScheme, PolynomialCommitmentType};
use sumcheck::ProverScratchPad;
use transcript::{
    BytesHashTranscript, FieldHashTranscript, Keccak256hasher, MIMCHasher, Proof, SHA256hasher,
    Transcript,
};

use crate::{gkr_prove, gkr_square_prove, RawCommitment};

#[cfg(feature = "grinding")]
pub(crate) fn grind<C: GKRConfig, T: Transcript<C::ChallengeField>>(
    transcript: &mut T,
    config: &Config<C>,
) {
    use arith::{Field, FieldSerde};

    let timer = start_timer!(|| format!("grind {} bits", config.grinding_bits));

    let mut hash_bytes = vec![];

    // ceil(32/field_size)
    let num_field_elements = (31 + C::ChallengeField::SIZE) / C::ChallengeField::SIZE;

    let initial_hash = transcript.generate_challenge_field_elements(num_field_elements);
    initial_hash
        .iter()
        .for_each(|h| h.serialize_into(&mut hash_bytes).unwrap()); // TODO: error propagation

    assert!(hash_bytes.len() >= 32, "hash len: {}", hash_bytes.len());
    hash_bytes.truncate(32);

    for _ in 0..(1 << config.grinding_bits) {
        transcript.append_u8_slice(&hash_bytes);
        hash_bytes = transcript.generate_challenge_u8_slice(32);
    }
    transcript.append_u8_slice(&hash_bytes[..32]);
    end_timer!(timer);
}

#[derive(Default)]
pub struct Prover<C: GKRConfig> {
    config: Config<C>,
    sp: ProverScratchPad<C>,
}

impl<C: GKRConfig> Prover<C> {
    pub fn new(config: &Config<C>) -> Self {
        // assert_eq!(config.fs_hash, crate::config::FiatShamirHashType::SHA256);
        assert_eq!(
            config.polynomial_commitment_type,
            PolynomialCommitmentType::Raw
        );
        Prover {
            config: config.clone(),
            sp: ProverScratchPad::default(),
        }
    }
    pub fn prepare_mem(&mut self, c: &Circuit<C>) {
        let max_num_input_var = c
            .layers
            .iter()
            .map(|layer| layer.input_var_num)
            .max()
            .unwrap();
        let max_num_output_var = c
            .layers
            .iter()
            .map(|layer| layer.output_var_num)
            .max()
            .unwrap();
        self.sp = ProverScratchPad::<C>::new(
            max_num_input_var,
            max_num_output_var,
            self.config.mpi_config.world_size(),
        );
    }

    fn prove_internal<T>(
        &mut self,
        c: &mut Circuit<C>,
        transcript: &mut T,
    ) -> (C::ChallengeField, Proof)
    where
        T: Transcript<C::ChallengeField>,
    {
        let timer = start_timer!(|| "prove");

        // PC commit
        let commitment =
            RawCommitment::<C>::mpi_new(&c.layers[0].input_vals, &self.config.mpi_config);

        let mut buffer = vec![];
        commitment.serialize_into(&mut buffer).unwrap(); // TODO: error propagation
        transcript.append_u8_slice(&buffer);

        self.config.mpi_config.transcript_sync_up(transcript);

        #[cfg(feature = "grinding")]
        grind::<C, T>(transcript, &self.config);

        c.fill_rnd_coefs(transcript);
        c.evaluate();

        let mut claimed_v = C::ChallengeField::default();
        let mut _rx = vec![];
        let mut _ry = None;
        let mut _rsimd = vec![];
        let mut _rmpi = vec![];

        if self.config.gkr_scheme == GKRScheme::GkrSquare {
            (_, _rx) = gkr_square_prove(c, &mut self.sp, transcript);
        } else {
            (claimed_v, _rx, _ry, _rsimd, _rmpi) =
                gkr_prove(c, &mut self.sp, transcript, &self.config.mpi_config);
        }

        // open
        match self.config.polynomial_commitment_type {
            PolynomialCommitmentType::Raw => {
                // no need to update transcript
            }
            _ => todo!(),
        }

        end_timer!(timer);

        (claimed_v, transcript.finalize_and_get_proof())
    }

    pub fn prove(&mut self, c: &mut Circuit<C>) -> (C::ChallengeField, Proof) {
        match C::FIAT_SHAMIR_HASH {
            FiatShamirHashType::Keccak256 => {
                let mut transcript =
                    BytesHashTranscript::<C::ChallengeField, Keccak256hasher>::new();
                self.prove_internal(c, &mut transcript)
            }
            FiatShamirHashType::SHA256 => {
                let mut transcript = BytesHashTranscript::<C::ChallengeField, SHA256hasher>::new();
                self.prove_internal(c, &mut transcript)
            }
            FiatShamirHashType::MIMC5 => {
                let mut transcript: FieldHashTranscript<<C as GKRConfig>::ChallengeField, _> =
                    FieldHashTranscript::<C::ChallengeField, MIMCHasher<C::ChallengeField>>::new();
                self.prove_internal(c, &mut transcript)
            }
            _ => unreachable!(),
        }
    }
}
