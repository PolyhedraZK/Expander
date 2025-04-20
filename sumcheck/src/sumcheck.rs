use std::io::Read;

use arith::Field;
use circuit::CircuitLayer;
use gkr_engine::{
    ExpanderDualVarChallenge, ExpanderSingleVarChallenge, FieldEngine, MPIConfig, MPIEngine,
    Transcript,
};
use serdes::ExpSerde;

use crate::{
    prover_helper::{SumcheckGkrSquareHelper, SumcheckGkrVanillaHelper},
    utils::transcript_io,
    GKRVerifierHelper, ProverScratchPad, VerifierScratchPad,
};

/// The degree of the polynomial for sumcheck, which is 2 for non-SIMD/MPI variables
/// and 3 for SIMD/MPI variables.
pub const SUMCHECK_GKR_DEGREE: usize = 2;
pub const SUMCHECK_GKR_SIMD_MPI_DEGREE: usize = 3;

/// The degree of the polynomial for sumcheck in the GKR square case.
/// It is 6 for both SIMD/MPI and non-SIMD/MPI variables.
pub const SUMCHECK_GKR_SQUARE_DEGREE: usize = 6;

// FIXME
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
// essentially the prev level of challenge passes here, once this level is done, new challenge gets
// written back into the prev space
pub fn sumcheck_prove_gkr_layer<F: FieldEngine, T: Transcript<F::ChallengeField>>(
    layer: &CircuitLayer<F>,
    challenge: &mut ExpanderDualVarChallenge<F>,
    alpha: Option<F::ChallengeField>,
    transcript: &mut T,
    sp: &mut ProverScratchPad<F>,
    mpi_config: &MPIConfig,
    is_output_layer: bool,
) {
    let mut helper =
        SumcheckGkrVanillaHelper::new(layer, challenge, alpha, sp, mpi_config, is_output_layer);

    helper.prepare_simd();
    helper.prepare_mpi();

    // gkr phase 1 over variable x
    helper.prepare_x_vals();
    for i_var in 0..helper.input_var_num {
        let evals = helper.poly_evals_at_rx(i_var, SUMCHECK_GKR_DEGREE);
        let r = transcript_io::<F::ChallengeField, T>(mpi_config, &evals, transcript);
        helper.receive_rx(i_var, r);
    }

    helper.prepare_simd_var_vals();
    for i_var in 0..helper.simd_var_num {
        let evals = helper.poly_evals_at_r_simd_var(i_var, SUMCHECK_GKR_SIMD_MPI_DEGREE);
        let r = transcript_io::<F::ChallengeField, T>(mpi_config, &evals, transcript);
        helper.receive_r_simd_var(i_var, r);
    }

    helper.prepare_mpi_var_vals();
    for i_var in 0..mpi_config.world_size().trailing_zeros() as usize {
        let evals = helper.poly_evals_at_r_mpi_var(i_var, SUMCHECK_GKR_SIMD_MPI_DEGREE);
        let r = transcript_io::<F::ChallengeField, T>(mpi_config, &evals, transcript);
        helper.receive_r_mpi_var(i_var, r);
    }

    let vx_claim = helper.vx_claim();
    transcript.append_field_element(&vx_claim);

    // gkr phase 2 over variable y
    if !layer.structure_info.skip_sumcheck_phase_two {
        helper.prepare_y_vals();
        for i_var in 0..helper.input_var_num {
            let evals = helper.poly_evals_at_ry(i_var, SUMCHECK_GKR_DEGREE);
            let r = transcript_io::<F::ChallengeField, T>(mpi_config, &evals, transcript);
            helper.receive_ry(i_var, r);
        }
        let vy_claim = helper.vy_claim();
        transcript.append_field_element(&vy_claim);
    }

    let rx = helper.rx;
    let ry = if !layer.structure_info.skip_sumcheck_phase_two {
        Some(helper.ry)
    } else {
        None
    };
    let r_simd = helper.r_simd_var;
    let r_mpi = helper.r_mpi_var;

    *challenge = ExpanderDualVarChallenge::new(rx, ry, r_simd, r_mpi);
}

// FIXME
#[allow(clippy::needless_range_loop)] // todo: remove
#[allow(clippy::type_complexity)]
pub fn sumcheck_prove_gkr_square_layer<F: FieldEngine, T: Transcript<F::ChallengeField>>(
    layer: &CircuitLayer<F>,
    challenge: &mut ExpanderSingleVarChallenge<F>,
    transcript: &mut T,
    sp: &mut ProverScratchPad<F>,
    mpi_config: &MPIConfig,
) {
    const D: usize = SUMCHECK_GKR_SQUARE_DEGREE + 1;
    let mut helper = SumcheckGkrSquareHelper::<F, D>::new(layer, challenge, sp, mpi_config);

    helper.prepare_simd();
    helper.prepare_mpi();
    helper.prepare_g_x_vals();

    // x-variable sumcheck rounds
    for i_var in 0..layer.input_var_num {
        let evals = helper.poly_evals_at_x(i_var);
        let r = transcript_io::<F::ChallengeField, T>(mpi_config, &evals, transcript);
        log::trace!("x i_var={} evals: {:?} r: {:?}", i_var, evals, r);
        helper.receive_x_challenge(i_var, r);
    }

    // Unpack SIMD witness polynomial evaluations
    helper.prepare_simd_var_vals();

    // SIMD-variable sumcheck rounds
    for i_var in 0..helper.simd_var_num {
        let evals = helper.poly_evals_at_simd(i_var);
        let r = transcript_io::<F::ChallengeField, T>(mpi_config, &evals, transcript);
        log::trace!("SIMD i_var={} evals: {:?} r: {:?}", i_var, evals, r);
        helper.receive_simd_challenge(i_var, r);
    }

    helper.prepare_mpi_var_vals();
    for i_var in 0..mpi_config.world_size().trailing_zeros() as usize {
        let evals = helper.poly_evals_at_mpi(i_var);
        let r = transcript_io::<F::ChallengeField, T>(mpi_config, &evals, transcript);
        helper.receive_mpi_challenge(i_var, r);
    }

    log::trace!("vx claim: {:?}", helper.vx_claim());
    transcript.append_field_element(&helper.vx_claim());

    *challenge = ExpanderSingleVarChallenge::new(helper.rx, helper.r_simd_var, helper.r_mpi_var);
}

#[inline(always)]
pub fn verify_sumcheck_step<F: FieldEngine>(
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

#[allow(clippy::too_many_arguments)]
pub fn sumcheck_verify_gkr_layer<F: FieldEngine>(
    proving_time_mpi_size: usize,
    layer: &CircuitLayer<F>,
    public_input: &[F::SimdCircuitField],
    challenge: &mut ExpanderDualVarChallenge<F>,
    claimed_v0: &mut F::ChallengeField,
    claimed_v1: &mut Option<F::ChallengeField>,
    alpha: Option<F::ChallengeField>,
    mut proof_reader: impl Read,
    transcript: &mut impl Transcript<F::ChallengeField>,
    sp: &mut VerifierScratchPad<F>,
    is_output_layer: bool,
) -> bool {
    assert_eq!(challenge.rz_1.is_none(), claimed_v1.is_none());
    assert_eq!(challenge.rz_1.is_none(), alpha.is_none());

    GKRVerifierHelper::prepare_layer(layer, &alpha, challenge, sp, is_output_layer);

    let var_num = layer.input_var_num;
    let simd_var_num = F::get_field_pack_size().trailing_zeros() as usize;
    let mut sum = *claimed_v0;
    if let Some(v1) = claimed_v1 {
        if let Some(a) = alpha {
            sum += *v1 * a;
        }
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
    }
    GKRVerifierHelper::set_r_simd_xy(&r_simd_xy, sp);

    for _i_var in 0..proving_time_mpi_size.trailing_zeros() {
        verified &= verify_sumcheck_step::<F>(
            &mut proof_reader,
            SUMCHECK_GKR_SIMD_MPI_DEGREE,
            transcript,
            &mut sum,
            &mut r_mpi_xy,
            sp,
        );
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

    *challenge = ExpanderDualVarChallenge::new(rx, ry, r_simd_xy, r_mpi_xy);
    *claimed_v0 = vx_claim;
    *claimed_v1 = vy_claim;

    verified
}
