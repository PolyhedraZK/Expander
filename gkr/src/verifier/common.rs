use std::{
    io::{Cursor, Read},
    vec,
};

use arith::Field;
use circuit::{Circuit, CircuitLayer};
use config::{Config, GKRConfig, GKRScheme};
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use poly_commit::{ExpanderGKRChallenge, PCSForExpanderGKR, StructuredReferenceString};
use serdes::ExpSerde;
use sumcheck::{
    GKRVerifierHelper, VerifierScratchPad, SUMCHECK_GKR_DEGREE, SUMCHECK_GKR_SIMD_MPI_DEGREE,
    SUMCHECK_GKR_SQUARE_DEGREE,
};
use transcript::{transcript_verifier_sync, Proof, Transcript};
use utils::timer::Timer;

#[inline(always)]
pub fn verify_sumcheck_step<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    mut proof_reader: impl Read,
    degree: usize,
    transcript: &mut T,
    claimed_sum: &mut C::ChallengeField,
    randomness_vec: &mut Vec<C::ChallengeField>,
    sp: &VerifierScratchPad<C>,
) -> bool {
    let mut ps = vec![];
    for i in 0..(degree + 1) {
        ps.push(C::ChallengeField::deserialize_from(&mut proof_reader).unwrap());
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
pub fn sumcheck_verify_gkr_layer<C: GKRFieldConfig, T: Transcript<C::ChallengeField>>(
    mpi_config: &MPIConfig,
    layer: &CircuitLayer<C>,
    public_input: &[C::SimdCircuitField],
    rz0: &[C::ChallengeField],
    rz1: &Option<Vec<C::ChallengeField>>,
    r_simd: &Vec<C::ChallengeField>,
    r_mpi: &Vec<C::ChallengeField>,
    claimed_v0: C::ChallengeField,
    claimed_v1: Option<C::ChallengeField>,
    alpha: Option<C::ChallengeField>,
    mut proof_reader: impl Read,
    transcript: &mut T,
    sp: &mut VerifierScratchPad<C>,
    is_output_layer: bool,
) -> (
    bool,
    Vec<C::ChallengeField>,
    Option<Vec<C::ChallengeField>>,
    Vec<C::ChallengeField>,
    Vec<C::ChallengeField>,
    C::ChallengeField,
    Option<C::ChallengeField>,
) {
    assert_eq!(rz1.is_none(), claimed_v1.is_none());
    assert_eq!(rz1.is_none(), alpha.is_none());

    GKRVerifierHelper::prepare_layer(layer, &alpha, rz0, rz1, r_simd, r_mpi, sp, is_output_layer);

    let var_num = layer.input_var_num;
    let simd_var_num = C::get_field_pack_size().trailing_zeros() as usize;
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
        verified &= verify_sumcheck_step::<C, T>(
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
        verified &= verify_sumcheck_step::<C, T>(
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
        verified &= verify_sumcheck_step::<C, T>(
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

    let vx_claim = C::ChallengeField::deserialize_from(&mut proof_reader).unwrap();

    sum -= vx_claim * GKRVerifierHelper::eval_add(&layer.add, sp);
    transcript.append_field_element(&vx_claim);

    let vy_claim = if !layer.structure_info.skip_sumcheck_phase_two {
        ry = Some(vec![]);
        for _i_var in 0..var_num {
            verified &= verify_sumcheck_step::<C, T>(
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

        let vy_claim = C::ChallengeField::deserialize_from(&mut proof_reader).unwrap();
        transcript.append_field_element(&vy_claim);
        verified &= sum == vx_claim * vy_claim * GKRVerifierHelper::eval_mul(&layer.mul, sp);
        Some(vy_claim)
    } else {
        verified &= sum == C::ChallengeField::ZERO;
        None
    };
    (verified, rx, ry, r_simd_xy, r_mpi_xy, vx_claim, vy_claim)
}
