//! This module implements the whole GKR prover, including the IOP and PCS.

use ark_std::{end_timer, start_timer};
use circuit::Circuit;
use config::{Config, GKRConfig, GKRScheme, PolynomialCommitmentType};
use gkr_field_config::GKRFieldConfig;
use sumcheck::ProverScratchPad;
use transcript::{transcript_root_broadcast, Proof, Transcript};

use crate::{gkr_prove, gkr_square_prove, RawCommitment};

#[cfg(feature = "grinding")]
pub(crate) fn grind<Cfg: GKRConfig>(transcript: &mut Cfg::Transcript, config: &Config<Cfg>) {
    use arith::{Field, FieldSerde};

    let timer = start_timer!(|| format!("grind {} bits", config.grinding_bits));

    let mut hash_bytes = vec![];

    // ceil(32/field_size)
    let num_field_elements = (31 + <Cfg::FieldConfig as GKRFieldConfig>::ChallengeField::SIZE)
        / <Cfg::FieldConfig as GKRFieldConfig>::ChallengeField::SIZE;

    let initial_hash = transcript.generate_challenge_field_elements(num_field_elements);
    initial_hash
        .iter()
        .for_each(|h| h.serialize_into(&mut hash_bytes).unwrap()); // TODO: error propagation

    assert!(hash_bytes.len() >= 32, "hash len: {}", hash_bytes.len());
    hash_bytes.truncate(32);

    transcript.lock_proof();
    for _ in 0..(1 << config.grinding_bits) {
        transcript.append_u8_slice(&hash_bytes);
        hash_bytes = transcript.generate_challenge_u8_slice(32);
    }
    transcript.append_u8_slice(&hash_bytes[..32]);
    transcript.unlock_proof();
    end_timer!(timer);
}

#[derive(Default)]
pub struct Prover<Cfg: GKRConfig> {
    config: Config<Cfg>,
    sp: ProverScratchPad<Cfg::FieldConfig>,
}

impl<Cfg: GKRConfig> Prover<Cfg> {
    pub fn new(config: &Config<Cfg>) -> Self {
        Prover {
            config: config.clone(),
            sp: ProverScratchPad::default(),
        }
    }

    pub fn prepare_mem(&mut self, c: &Circuit<Cfg::FieldConfig>) {
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
        self.sp = ProverScratchPad::<Cfg::FieldConfig>::new(
            max_num_input_var,
            max_num_output_var,
            self.config.mpi_config.world_size(),
        );
    }

    pub fn prove(
        &mut self,
        c: &mut Circuit<Cfg::FieldConfig>,
    ) -> (<Cfg::FieldConfig as GKRFieldConfig>::ChallengeField, Proof) {
        let timer = start_timer!(|| "prove");
        let mut transcript = Cfg::Transcript::new();

        // PC commit
        let commitment = RawCommitment::<Cfg::FieldConfig>::mpi_new(
            &c.layers[0].input_vals,
            &self.config.mpi_config,
        );

        let mut buffer = vec![];
        commitment.serialize_into(&mut buffer).unwrap(); // TODO: error propagation
        transcript.append_u8_slice(&buffer);

        transcript_root_broadcast(&mut transcript, &self.config.mpi_config);

        #[cfg(feature = "grinding")]
        grind::<Cfg>(&mut transcript, &self.config);

        c.fill_rnd_coefs(&mut transcript);
        c.evaluate();

        let mut claimed_v = <Cfg::FieldConfig as GKRFieldConfig>::ChallengeField::default();
        let mut _rx = vec![];
        let mut _ry = None;
        let mut _rsimd = vec![];
        let mut _rmpi = vec![];

        if self.config.gkr_scheme == GKRScheme::GkrSquare {
            (_, _rx) = gkr_square_prove(c, &mut self.sp, &mut transcript);
        } else {
            (claimed_v, _rx, _ry, _rsimd, _rmpi) =
                gkr_prove(c, &mut self.sp, &mut transcript, &self.config.mpi_config);
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
}
