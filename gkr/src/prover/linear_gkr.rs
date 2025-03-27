//! This module implements the whole GKR prover, including the IOP and PCS.

use arith::Field;
use circuit::Circuit;
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, GKREngine, GKRScheme, MPIConfig,
    MPIEngine, Proof, StructuredReferenceString, Transcript,
};
use polynomials::{MultilinearExtension, MutRefMultiLinearPoly};
use serdes::ExpSerde;
use sumcheck::ProverScratchPad;
use transcript::transcript_root_broadcast;
use utils::timer::Timer;

use crate::{gkr_prove, gkr_square_prove};

#[cfg(feature = "grinding")]
pub(crate) fn grind<Cfg: GKREngine>(
    transcript: &mut impl Transcript<<Cfg::FieldConfig as FieldEngine>::ChallengeField>,
    mpi_config: &MPIConfig,
) {
    use crate::GRINDING_BITS;

    let timer = Timer::new("grinding", mpi_config.is_root());

    let mut hash_bytes = vec![];

    // ceil(32/field_size)
    let num_field_elements = (31 + <Cfg::FieldConfig as FieldEngine>::ChallengeField::SIZE)
        / <Cfg::FieldConfig as FieldEngine>::ChallengeField::SIZE;

    let initial_hash = transcript.generate_challenge_field_elements(num_field_elements);
    initial_hash
        .iter()
        .for_each(|h| h.serialize_into(&mut hash_bytes).unwrap()); // TODO: error propagation

    assert!(hash_bytes.len() >= 32, "hash len: {}", hash_bytes.len());
    hash_bytes.truncate(32);

    transcript.lock_proof();
    for _ in 0..(1 << GRINDING_BITS) {
        transcript.append_u8_slice(&hash_bytes);
        hash_bytes = transcript.generate_challenge_u8_slice(32);
    }
    transcript.append_u8_slice(&hash_bytes[..32]);
    transcript.unlock_proof();
    timer.stop();
}

#[derive(Default)]
pub struct Prover<Cfg: GKREngine> {
    pub mpi_config: MPIConfig,
    sp: ProverScratchPad<Cfg::FieldConfig>,
}

impl<Cfg: GKREngine> Prover<Cfg> {
    pub fn new(mpi_config: MPIConfig) -> Self {
        Prover {
            mpi_config,
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
            self.mpi_config.world_size(),
        );
    }

    pub fn prove(
        &mut self,
        c: &mut Circuit<Cfg::FieldConfig>,
        pcs_params: &<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Params,
        pcs_proving_key: &<<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::SRS as StructuredReferenceString>::PKey,
        pcs_scratch: &mut <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::ScratchPad,
    ) -> (<Cfg::FieldConfig as FieldEngine>::ChallengeField, Proof) {
        let proving_timer = Timer::new("prover", self.mpi_config.is_root());
        let mut transcript = Cfg::TranscriptConfig::new();

        let pcs_commit_timer = Timer::new("pcs commit", self.mpi_config.is_root());
        // PC commit
        let commitment = {
            let original_input_vars = c.log_input_size();
            let mut mle_ref = MutRefMultiLinearPoly::from_ref(&mut c.layers[0].input_vals);
            if original_input_vars < Cfg::PCSConfig::MINIMUM_NUM_VARS {
                eprintln!(
					"{} over {} has minimum supported local vars {}, but input poly has vars {}, pad to {} vars in commiting.",
					Cfg::PCSConfig::NAME,
					<Cfg::FieldConfig as FieldEngine>::SimdCircuitField::NAME,
					Cfg::PCSConfig::MINIMUM_NUM_VARS,
					original_input_vars,
					Cfg::PCSConfig::MINIMUM_NUM_VARS,
				);
                mle_ref.lift_to_n_vars(Cfg::PCSConfig::MINIMUM_NUM_VARS)
            }

            let commit = Cfg::PCSConfig::commit(
                pcs_params,
                &self.mpi_config,
                pcs_proving_key,
                &mle_ref,
                pcs_scratch,
            );

            mle_ref.lift_to_n_vars(original_input_vars);

            commit
        };

        if self.mpi_config.is_root() {
            let mut buffer = vec![];
            commitment.unwrap().serialize_into(&mut buffer).unwrap(); // TODO: error propagation
            transcript.append_commitment(&buffer);
        }
        pcs_commit_timer.stop();

        transcript_root_broadcast(&mut transcript, &self.mpi_config);

        #[cfg(feature = "grinding")]
        grind::<Cfg>(&mut transcript, &self.mpi_config);

        c.fill_rnd_coefs(&mut transcript);
        c.evaluate();

        let (claimed_v, rx, rsimd, rmpi);
        let mut ry = None;

        let gkr_prove_timer = Timer::new("gkr prove", self.mpi_config.is_root());
        if Cfg::SCHEME == GKRScheme::GkrSquare {
            (claimed_v, rx, rsimd, rmpi) =
                gkr_square_prove(c, &mut self.sp, &mut transcript, &self.mpi_config);
        } else {
            (claimed_v, rx, ry, rsimd, rmpi) =
                gkr_prove(c, &mut self.sp, &mut transcript, &self.mpi_config);
        }
        gkr_prove_timer.stop();

        transcript_root_broadcast(&mut transcript, &self.mpi_config);

        let pcs_open_timer = Timer::new("pcs open", self.mpi_config.is_root());
        // open
        let mut mle_ref = MutRefMultiLinearPoly::from_ref(&mut c.layers[0].input_vals);
        self.prove_input_layer_claim(
            &mut mle_ref,
            &mut ExpanderSingleVarChallenge {
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
            transcript_root_broadcast(&mut transcript, &self.mpi_config);
            self.prove_input_layer_claim(
                &mut mle_ref,
                &mut ExpanderSingleVarChallenge {
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

impl<Cfg: GKREngine> Prover<Cfg> {
    fn prove_input_layer_claim(
        &self,
        inputs: &mut MutRefMultiLinearPoly<<Cfg::FieldConfig as FieldEngine>::SimdCircuitField>,
        open_at: &mut ExpanderSingleVarChallenge<Cfg::FieldConfig>,
        pcs_params: &<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::Params,
        pcs_proving_key: &<<Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::SRS as StructuredReferenceString>::PKey,
        pcs_scratch: &mut <Cfg::PCSConfig as ExpanderPCS<Cfg::FieldConfig>>::ScratchPad,
        transcript: &mut impl Transcript<<Cfg::FieldConfig as FieldEngine>::ChallengeField>,
    ) {
        let original_input_vars = inputs.num_vars();
        if original_input_vars < Cfg::PCSConfig::MINIMUM_NUM_VARS {
            eprintln!(
				"{} over {} has minimum supported local vars {}, but input poly has vars {}, pad to {} vars in opening.",
				Cfg::PCSConfig::NAME,
				<Cfg::FieldConfig as FieldEngine>::SimdCircuitField::NAME,
				Cfg::PCSConfig::MINIMUM_NUM_VARS,
				original_input_vars,
				Cfg::PCSConfig::MINIMUM_NUM_VARS,
			);
            inputs.lift_to_n_vars(Cfg::PCSConfig::MINIMUM_NUM_VARS);
            open_at.x.resize(
                Cfg::PCSConfig::MINIMUM_NUM_VARS,
                <Cfg::FieldConfig as FieldEngine>::ChallengeField::ZERO,
            )
        }

        transcript.lock_proof();
        let opening = Cfg::PCSConfig::open(
            pcs_params,
            &self.mpi_config,
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
            <Cfg::FieldConfig as FieldEngine>::ChallengeField::ZERO,
        );

        if self.mpi_config.is_root() {
            let mut buffer = vec![];
            opening.unwrap().serialize_into(&mut buffer).unwrap(); // TODO: error propagation
            transcript.append_u8_slice(&buffer);
        }
    }
}
