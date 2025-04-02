use circuit::Circuit;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use polynomials::MultiLinearPolyExpander;
use serdes::ExpSerde;
use sumcheck::{sumcheck_prove_gkr_layer, ProverScratchPad};
use transcript::Transcript;
use utils::timer::Timer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SumcheckLayerState<C: GKRFieldConfig> {
    pub transcript_state: Vec<u8>,
    pub rz0: Vec<C::ChallengeField>,
    pub rz1: Option<Vec<C::ChallengeField>>,
    pub r_simd: Vec<C::ChallengeField>,
    pub r_mpi: Vec<C::ChallengeField>,
    pub alpha: Option<C::ChallengeField>,
    pub claimed_v0: C::ChallengeField,
    pub claimed_v1: Option<C::ChallengeField>,
}

impl<C: GKRFieldConfig> ExpSerde for SumcheckLayerState<C> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> serdes::SerdeResult<()> {
        self.transcript_state.serialize_into(&mut writer)?;
        self.rz0.serialize_into(&mut writer)?;
        self.rz1.serialize_into(&mut writer)?;
        self.r_simd.serialize_into(&mut writer)?;
        self.r_mpi.serialize_into(&mut writer)?;
        self.alpha.serialize_into(&mut writer)?;
        self.claimed_v0.serialize_into(&mut writer)?;
        self.claimed_v1.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> serdes::SerdeResult<Self> {
        let transcript_state = Vec::deserialize_from(&mut reader)?;
        let rz0 = Vec::deserialize_from(&mut reader)?;
        let rz1 = Option::<Vec<C::ChallengeField>>::deserialize_from(&mut reader)?;
        let r_simd = Vec::deserialize_from(&mut reader)?;
        let r_mpi = Vec::deserialize_from(&mut reader)?;
        let alpha = Option::<C::ChallengeField>::deserialize_from(&mut reader)?;
        let claimed_v0 = C::ChallengeField::deserialize_from(&mut reader)?;
        let claimed_v1 = Option::<C::ChallengeField>::deserialize_from(&mut reader)?;

        Ok(Self {
            transcript_state,
            rz0,
            rz1,
            r_simd,
            r_mpi,
            alpha,
            claimed_v0,
            claimed_v1,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn checkpoint_sumcheck_layer_state<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    rz0: &[C::ChallengeField],
    rz1: &Option<Vec<C::ChallengeField>>,
    r_simd: &[C::ChallengeField],
    r_mpi: &[C::ChallengeField],
    alpha: &Option<C::ChallengeField>,
    claimed_v0: &C::ChallengeField,
    claimed_v1: &Option<C::ChallengeField>,
    transcript: &mut T,
    mpi_config: &MPIConfig,
) {
    let transcript_state = transcript.hash_and_return_state();
    if mpi_config.is_root() {
        let sumcheck_state = SumcheckLayerState::<C> {
            transcript_state: transcript_state.clone(),
            rz0: rz0.to_vec(),
            rz1: rz1.clone(),
            r_simd: r_simd.to_vec(),
            r_mpi: r_mpi.to_vec(),
            alpha: *alpha,
            claimed_v0: *claimed_v0,
            claimed_v1: *claimed_v1,
        };
        let mut state_bytes = vec![];
        sumcheck_state.serialize_into(&mut state_bytes).unwrap();
        transcript.set_state(&transcript_state);
    }
}

#[allow(clippy::type_complexity)]
pub fn gkr_par_verifier_prove<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    circuit: &Circuit<C>,
    sp: &mut ProverScratchPad<C>,
    transcript: &mut T,
    mpi_config: &MPIConfig,
) -> (
    C::ChallengeField,
    Vec<C::ChallengeField>,
    Option<Vec<C::ChallengeField>>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
) {
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

    for _ in 0..mpi_config.world_size().trailing_zeros() {
        r_mpi.push(transcript.generate_challenge_field_element());
    }

    let mut alpha = None;

    let output_vals = &circuit.layers.last().unwrap().output_vals;
    let claimed_v =
        MultiLinearPolyExpander::<C>::collectively_eval_circuit_vals_at_expander_challenge(
            output_vals,
            &rz0,
            &r_simd,
            &r_mpi,
            &mut sp.hg_evals,
            &mut sp.eq_evals_first_half, // confusing name here..
            mpi_config,
        );

    let mut claimed_v0 = claimed_v;
    let mut claimed_v1 = None;
    for i in (0..layer_num).rev() {
        let timer = Timer::new(
            &format!(
                "Sumcheck Layer {}, n_vars {}, one phase only? {}",
                i,
                &circuit.layers[i].input_var_num,
                &circuit.layers[i].structure_info.skip_sumcheck_phase_two,
            ),
            mpi_config.is_root(),
        );

        checkpoint_sumcheck_layer_state::<C, _>(
            &rz0,
            &rz1,
            &r_simd,
            &r_mpi,
            &alpha,
            &claimed_v0,
            &claimed_v1,
            transcript,
            mpi_config,
        );

        (rz0, rz1, r_simd, r_mpi, claimed_v0, claimed_v1) = sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &rz0,
            &rz1,
            &r_simd,
            &r_mpi,
            alpha,
            transcript,
            sp,
            mpi_config,
            i == layer_num - 1,
        );

        if rz1.is_some() {
            let mut tmp = transcript.generate_challenge_field_element();
            mpi_config.root_broadcast_f(&mut tmp);
            alpha = Some(tmp)
        } else {
            alpha = None;
        }
        timer.stop();
    }

    (claimed_v, rz0, rz1, r_simd, r_mpi)
}
