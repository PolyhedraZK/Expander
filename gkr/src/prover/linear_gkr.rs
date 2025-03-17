//! This module implements the whole GKR prover, including the IOP and PCS.

use arith::{Field, FieldSerde};
use circuit::Circuit;
use config::{Config, GKRConfig, GKRScheme};
use gkr_field_config::GKRFieldConfig;
use poly_commit::{ExpanderGKRChallenge, PCSForExpanderGKR, StructuredReferenceString};
use polynomials::{MultilinearExtension, MutRefMultiLinearPoly};
use sumcheck::ProverScratchPad;
use transcript::{transcript_root_broadcast, Proof, Transcript};
use utils::timer::Timer;

use crate::{gkr_prove, gkr_square_prove};

#[cfg(feature = "grinding")]
pub(crate) fn grind<Cfg: GKRConfig>(transcript: &mut Cfg::Transcript, config: &Config<Cfg>) {
    use arith::{Field, FieldSerde};

    let timer = Timer::new("grinding", config.mpi_config.is_root());

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
    timer.stop();
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
        pcs_params: &<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::Params,
        pcs_proving_key: &<<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::SRS as StructuredReferenceString>::PKey,
        pcs_scratch: &mut <Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::ScratchPad,
    ) -> (<Cfg::FieldConfig as GKRFieldConfig>::ChallengeField, Proof) {
        let proving_timer = Timer::new("prover", self.config.mpi_config.is_root());
        let mut transcript = Cfg::Transcript::new();

        let pcs_commit_timer = Timer::new("pcs commit", self.config.mpi_config.is_root());
        // PC commit
        let commitment = {
            let original_input_vars = c.log_input_size();
            let mut mle_ref = MutRefMultiLinearPoly::from_ref(&mut c.layers[0].input_vals);
            if original_input_vars < Cfg::PCS::MINIMUM_NUM_VARS {
                mle_ref.lift_to_n_vars(Cfg::PCS::MINIMUM_NUM_VARS)
            }

            let commit = Cfg::PCS::commit(
                pcs_params,
                &self.config.mpi_config,
                pcs_proving_key,
                &mle_ref,
                pcs_scratch,
            );

            mle_ref.lift_to_n_vars(original_input_vars);

            commit
        };
        let mut buffer = vec![];
        commitment.serialize_into(&mut buffer).unwrap(); // TODO: error propagation
        transcript.append_commitment(&buffer);
        pcs_commit_timer.stop();

        transcript_root_broadcast(&mut transcript, &self.config.mpi_config);

        #[cfg(feature = "grinding")]
        grind::<Cfg>(&mut transcript, &self.config);

        c.fill_rnd_coefs(&mut transcript);
        c.evaluate();

        let (claimed_v, rx, rsimd, rmpi);
        let mut ry = None;

        let gkr_prove_timer = Timer::new("gkr prove", self.config.mpi_config.is_root());
        if self.config.gkr_scheme == GKRScheme::GkrSquare {
            (claimed_v, rx, rsimd, rmpi) =
                gkr_square_prove(c, &mut self.sp, &mut transcript, &self.config.mpi_config);
        } else {
            (claimed_v, rx, ry, rsimd, rmpi) =
                gkr_prove(c, &mut self.sp, &mut transcript, &self.config.mpi_config);
        }
        gkr_prove_timer.stop();

        transcript_root_broadcast(&mut transcript, &self.config.mpi_config);

        let pcs_open_timer = Timer::new("pcs open", self.config.mpi_config.is_root());
        // open
        let mut mle_ref = MutRefMultiLinearPoly::from_ref(&mut c.layers[0].input_vals);
        self.prove_input_layer_claim(
            &mut mle_ref,
            &mut ExpanderGKRChallenge {
                x: rx,
                x_simd: rsimd.clone(),
                x_mpi: rmpi.clone(),
            },
            pcs_params,
            pcs_proving_key,
            pcs_scratch,
            &mut transcript,
        );

        if let Some(ry) = ry {
            transcript_root_broadcast(&mut transcript, &self.config.mpi_config);
            self.prove_input_layer_claim(
                &mut mle_ref,
                &mut ExpanderGKRChallenge {
                    x: ry,
                    x_simd: rsimd,
                    x_mpi: rmpi,
                },
                pcs_params,
                pcs_proving_key,
                pcs_scratch,
                &mut transcript,
            );
        }

        pcs_open_timer.stop();

        let proof = transcript.finalize_and_get_proof();
        proving_timer.print(&format!("Proof size {} bytes", proof.bytes.len()));
        proving_timer.stop();

        (claimed_v, proof)
    }
}

impl<Cfg: GKRConfig> Prover<Cfg> {
    fn prove_input_layer_claim(
        &self,
        inputs: &mut MutRefMultiLinearPoly<<Cfg::FieldConfig as GKRFieldConfig>::SimdCircuitField>,
        open_at: &mut ExpanderGKRChallenge<Cfg::FieldConfig>,
        pcs_params: &<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::Params,
        pcs_proving_key: &<<Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::SRS as StructuredReferenceString>::PKey,
        pcs_scratch: &mut <Cfg::PCS as PCSForExpanderGKR<Cfg::FieldConfig, Cfg::Transcript>>::ScratchPad,
        transcript: &mut Cfg::Transcript,
    ) {
        let original_input_vars = inputs.num_vars();
        if original_input_vars < Cfg::PCS::MINIMUM_NUM_VARS {
            inputs.lift_to_n_vars(Cfg::PCS::MINIMUM_NUM_VARS);
            open_at.x.resize(
                Cfg::PCS::MINIMUM_NUM_VARS,
                <Cfg::FieldConfig as GKRFieldConfig>::ChallengeField::ZERO,
            )
        }

        transcript.lock_proof();
        let opening = Cfg::PCS::open(
            pcs_params,
            &self.config.mpi_config,
            pcs_proving_key,
            inputs,
            open_at,
            transcript,
            pcs_scratch,
        );
        transcript.unlock_proof();

        inputs.lift_to_n_vars(original_input_vars);
        open_at.x.resize(
            original_input_vars,
            <Cfg::FieldConfig as GKRFieldConfig>::ChallengeField::ZERO,
        );

        let mut buffer = vec![];
        opening.serialize_into(&mut buffer).unwrap(); // TODO: error propagation
        transcript.append_u8_slice(&buffer);
    }
}
