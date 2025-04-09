use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use gkr_engine::{MPIEngine, Transcript};
use polynomials::{EqPolynomial, MultilinearExtension};

use crate::{
    orion::{
        mpi_utils::{mpi_commit_encoded, orion_mpi_compute_mt_root, orion_mpi_mt_openings},
        utils::{lut_open_linear_combine, simd_open_linear_combine},
        OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    },
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

#[inline(always)]
pub fn orion_mpi_commit_simd_field<F, SimdF, ComPackF>(
    mpi_engine: &impl MPIEngine,
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    scratch_pad: &mut OrionScratchPad,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = {
        let local_num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        let (row_field_elems, msg_size) = OrionSRS::local_eval_shape(
            mpi_engine.world_size(),
            local_num_vars,
            F::FIELD_SIZE,
            ComPackF::PACK_SIZE,
        );
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let relative_pack_size = ComPackF::PACK_SIZE / SimdF::PACK_SIZE;
    assert_eq!(ComPackF::PACK_SIZE % SimdF::PACK_SIZE, 0);

    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    assert_eq!(poly.hypercube_size() % relative_pack_size, 0);
    let packed_evals_ref = unsafe {
        let ptr = poly.hypercube_basis_ref().as_ptr();
        let len = poly.hypercube_size() / relative_pack_size;
        std::slice::from_raw_parts(ptr as *const ComPackF, len)
    };

    let local_commitment = mpi_commit_encoded(
        mpi_engine,
        pk,
        packed_evals_ref,
        scratch_pad,
        packed_rows,
        msg_size,
    )?;

    orion_mpi_compute_mt_root(mpi_engine, local_commitment, scratch_pad)
}

#[inline(always)]
pub fn orion_mpi_open_simd_field<F, SimdF, EvalF, ComPackF>(
    mpi_engine: &impl MPIEngine,
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    point: &[EvalF],
    mpi_point: &[EvalF],
    transcript: &mut impl Transcript<EvalF>,
    scratch_pad: &OrionScratchPad,
) -> Option<OrionProof<EvalF>>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (local_row_field_elems, msg_size) = {
        let local_num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        assert_eq!(local_num_vars, point.len());

        let (local_row_field_elems, msg_size) = OrionSRS::local_eval_shape(
            mpi_engine.world_size(),
            local_num_vars,
            F::FIELD_SIZE,
            ComPackF::PACK_SIZE,
        );
        (local_row_field_elems, msg_size)
    };

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: pre-compute the eq linear combine coeffs for linear combination
    let eq_col_coeffs = {
        let mut eq_vars = point[..num_vars_in_com_simd].to_vec();
        eq_vars.extend_from_slice(&point[num_vars_in_com_simd + num_vars_in_msg..]);
        let mut coeffs = EqPolynomial::build_eq_x_r(&eq_vars);
        let mpi_weight = EqPolynomial::ith_eq_vec_elem(mpi_point, mpi_engine.world_rank());
        coeffs.iter_mut().for_each(|c| *c *= mpi_weight);
        coeffs
    };

    // NOTE: pre-declare the spaces for returning evaluation and proximity queries
    let mut eval_row = vec![EvalF::ZERO; msg_size];

    let proximity_test_num = pk.proximity_repetitions::<EvalF>(PCS_SOUNDNESS_BITS);
    let mut proximity_rows = vec![vec![EvalF::ZERO; msg_size]; proximity_test_num];

    // NOTE: draw randomness from transcript with log random complexity
    let num_of_local_random_vars = point.len() - num_vars_in_msg;
    let local_random_coeffs: Vec<_> = (0..proximity_test_num)
        .map(|_| {
            let local_rand = transcript.generate_challenge_field_elements(num_of_local_random_vars);
            let mpi_rand = transcript.generate_challenge_field_elements(mpi_point.len());
            let mut coeffs = EqPolynomial::build_eq_x_r(&local_rand);
            let mpi_weight = EqPolynomial::ith_eq_vec_elem(&mpi_rand, mpi_engine.world_rank());
            coeffs.iter_mut().for_each(|c| *c *= mpi_weight);
            coeffs
        })
        .collect();

    match F::NAME {
        GF2::NAME => lut_open_linear_combine(
            ComPackF::PACK_SIZE,
            poly.hypercube_basis_ref(),
            &eq_col_coeffs,
            &mut eval_row,
            &local_random_coeffs,
            &mut proximity_rows,
        ),
        _ => simd_open_linear_combine(
            ComPackF::PACK_SIZE,
            poly.hypercube_basis_ref(),
            &eq_col_coeffs,
            &mut eval_row,
            &local_random_coeffs,
            &mut proximity_rows,
        ),
    }

    // NOTE: MPI sum up local weighed rows
    eval_row = mpi_engine.sum_vec(&eval_row);
    proximity_rows = proximity_rows
        .iter()
        .map(|r| mpi_engine.sum_vec(r))
        .collect();

    // NOTE: MT opening for point queries
    let num_leaves_per_opening = {
        let num_bits_opening = local_row_field_elems * F::FIELD_SIZE * mpi_engine.world_size();
        let num_bytes_opening = num_bits_opening / 8;

        num_bytes_opening / tree::LEAF_BYTES
    };
    let query_openings = orion_mpi_mt_openings(
        mpi_engine,
        pk,
        num_leaves_per_opening,
        scratch_pad,
        transcript,
    );

    if !mpi_engine.is_root() {
        return None;
    }

    OrionProof {
        eval_row,
        proximity_rows,
        query_openings: query_openings.unwrap(),
        merkle_cap: scratch_pad.merkle_cap.clone(),
    }
    .into()
}
