use arith::{ExtensionField, Field, SimdField};
use gf2::GF2;
use mpi_config::MPIConfig;
use polynomials::{EqPolynomial, MultilinearExtension};
use transcript::Transcript;

use crate::{
    orion::{
        mpi_utils::{mpi_commit_encoded, orion_mpi_compute_mt_root, orion_mpi_mt_openings},
        utils::{lut_open_linear_combine, pack_simd, simd_open_linear_combine},
        OrionCommitment, OrionProof, OrionResult, OrionSRS, OrionScratchPad,
    },
    traits::TensorCodeIOPPCS,
    PCS_SOUNDNESS_BITS,
};

#[inline(always)]
pub(crate) fn orion_mpi_commit_simd_field<F, SimdF, ComPackF>(
    mpi_config: &MPIConfig,
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    scratch_pad: &mut OrionScratchPad<F, ComPackF>,
) -> OrionResult<OrionCommitment>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    ComPackF: SimdField<Scalar = F>,
{
    let (row_num, msg_size) = {
        let num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        let (row_field_elems, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        let row_num = row_field_elems / SimdF::PACK_SIZE;
        (row_num, msg_size)
    };

    let relative_pack_size = ComPackF::PACK_SIZE / SimdF::PACK_SIZE;
    assert_eq!(ComPackF::PACK_SIZE % SimdF::PACK_SIZE, 0);

    let packed_rows = row_num / relative_pack_size;
    assert_eq!(row_num % relative_pack_size, 0);

    assert_eq!(poly.hypercube_size() % relative_pack_size, 0);
    let packed_evals = pack_simd::<F, SimdF, ComPackF>(poly.hypercube_basis_ref());

    let local_commitment = mpi_commit_encoded(
        mpi_config,
        pk,
        &packed_evals,
        scratch_pad,
        packed_rows,
        msg_size,
    )?;

    orion_mpi_compute_mt_root(mpi_config, local_commitment, scratch_pad)
}

#[inline(always)]
pub(crate) fn orion_mpi_open_simd_field<F, SimdF, EvalF, ComPackF, T>(
    mpi_config: &MPIConfig,
    pk: &OrionSRS,
    poly: &impl MultilinearExtension<SimdF>,
    point: &[EvalF],
    mpi_point: &[EvalF],
    transcript: &mut T,
    scratch_pad: &OrionScratchPad<F, ComPackF>,
) -> Option<OrionProof<EvalF>>
where
    F: Field,
    SimdF: SimdField<Scalar = F>,
    EvalF: ExtensionField<BaseField = F>,
    ComPackF: SimdField<Scalar = F>,
    T: Transcript<EvalF>,
{
    let msg_size = {
        let num_vars = poly.num_vars() + SimdF::PACK_SIZE.ilog2() as usize;
        assert_eq!(num_vars, point.len());

        let (_, msg_size) = OrionSRS::evals_shape::<F>(num_vars);
        msg_size
    };

    let num_vars_in_com_simd = ComPackF::PACK_SIZE.ilog2() as usize;
    let num_vars_in_msg = msg_size.ilog2() as usize;

    // NOTE: pre-compute the eq linear combine coeffs for linear combination
    let eq_col_coeffs = {
        let mut eq_vars = point[..num_vars_in_com_simd].to_vec();
        eq_vars.extend_from_slice(&point[num_vars_in_com_simd + num_vars_in_msg..]);
        let mut coeffs = EqPolynomial::build_eq_x_r(&eq_vars);
        let mpi_weight = EqPolynomial::ith_eq_vec_elem(mpi_point, mpi_config.world_rank());
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
            let mpi_weight = EqPolynomial::ith_eq_vec_elem(&mpi_rand, mpi_config.world_rank());
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
    eval_row = mpi_config.sum_vec(&eval_row);
    proximity_rows = proximity_rows
        .iter()
        .map(|r| mpi_config.sum_vec(r))
        .collect();

    // NOTE: MT opening for point queries
    let query_openings = orion_mpi_mt_openings(mpi_config, pk, scratch_pad, transcript);

    if !mpi_config.is_root() {
        return None;
    }

    OrionProof {
        eval_row,
        proximity_rows,
        query_openings: query_openings.unwrap(),
    }
    .into()
}
