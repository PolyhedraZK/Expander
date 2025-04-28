use arith::Field;
use ark_std::test_rng;
use gkr_engine::{ExpanderPCS, FieldEngine, MPIConfig, MPIEngine, StructuredReferenceString};
use transpose::transpose_inplace;

#[allow(clippy::type_complexity)]
pub fn expander_pcs_init_testing_only<FieldConfig: FieldEngine, PCS: ExpanderPCS<FieldConfig>>(
    n_input_vars: usize,
    mpi_config: &MPIConfig,
) -> (
    PCS::Params,
    <PCS::SRS as StructuredReferenceString>::PKey,
    <PCS::SRS as StructuredReferenceString>::VKey,
    PCS::ScratchPad,
) {
    let mut rng = test_rng();

    let mut pcs_params = <PCS as ExpanderPCS<FieldConfig>>::gen_params(n_input_vars);
    let (pcs_setup, calibrated_num_local_simd_vars) =
        <PCS as ExpanderPCS<FieldConfig>>::gen_srs_for_testing(&pcs_params, mpi_config, &mut rng);

    if n_input_vars < calibrated_num_local_simd_vars {
        eprintln!(
            "{} over {} has minimum supported local vars {}, but input poly has vars {}.",
            PCS::NAME,
            FieldConfig::SimdCircuitField::NAME,
            calibrated_num_local_simd_vars,
            n_input_vars,
        );
        pcs_params = <PCS as ExpanderPCS<FieldConfig>>::gen_params(calibrated_num_local_simd_vars);
    }

    let (pcs_proving_key, pcs_verification_key) = pcs_setup.into_keys();
    let pcs_scratch = <PCS as ExpanderPCS<FieldConfig>>::init_scratch_pad(&pcs_params, mpi_config);

    (
        pcs_params,
        pcs_proving_key,
        pcs_verification_key,
        pcs_scratch,
    )
}

#[inline(always)]
pub(crate) fn mpi_matrix_transpose<F: Sized + Copy + Clone + Default>(
    mpi_engine: &impl MPIEngine,
    local_matrix: &mut [F],
    local_col_size: usize,
) {
    assert_eq!(local_matrix.len() % local_col_size, 0);
    assert!(local_matrix.len() / local_col_size >= mpi_engine.world_size());
    assert!(local_matrix.len().is_power_of_two());

    /*
    The input should be in column order, with column length being local_col_size

    p(0):     * * * * * * * * * *  ....  *
              |/|/|/|/|/|/|/|/|/|  .... /|
              * * * * * * * * * *  ....  *

    p(1):     * * * * * * * * * *  ....  *
              |/|/|/|/|/|/|/|/|/|  .... /|
              * * * * * * * * * *  ....  *

    ...

    p(n - 1): * * * * * * * * * *  ....  *
              |/|/|/|/|/|/|/|/|/|  .... /|
              * * * * * * * * * *  ....  *

    A global MPI ALL TO ALL rewinds the order into the following:

    p(0)        p(1)        p(2)     p(n - 1)
    * * * *     * * * *     * *  ....  *
    |/|/|/|     |/|/|/|     |/|  .... /|
    * * * *     * * * *     * *  ....  *
         /           /
        /           /
       /           /
      /           /
     /           /           /
    * * * *     * * * *     * *  ....  *
    |/|/|/|     |/|/|/|     |/|  .... /|
    * * * *     * * * *     * *  ....  *
         /           /
        /           /
       /           /
      /           /
     /           /           /
    * * * *     * * * *     * *  ....  *
    |/|/|/|     |/|/|/|     |/|  .... /|
    * * * *     * * * *     * *  ....  *

     */

    // NOTE: ALL-TO-ALL transpose go get other world's slice of columns
    mpi_engine.all_to_all_transpose(local_matrix);

    /*
    Rearrange each row of interleaved codeword on each process, we have:

    p(0)        p(1)        p(2)     p(n - 1)
    *-*-*-*     *-*-*-*     *-*- .... -*
    *-*-*-*     *-*-*-*     *-*- .... -*
         /           /
        /           /
       /           /
      /           /
     /           /           /
    *-*-*-*     *-*-*-*     *-*- .... -*
    *-*-*-*     *-*-*-*     *-*- .... -*
         /           /
        /           /
       /           /
      /           /
     /           /           /
    *-*-*-*     *-*-*-*     *-*- .... -*
    *-*-*-*     *-*-*-*     *-*- .... -*

     */

    let global_col_size = mpi_engine.world_size() * local_col_size;
    let row_length = local_matrix.len() / global_col_size;

    if local_col_size > 1 {
        let sub_matrix_per_world_len = local_col_size * row_length;

        // NOTE: now transpose back to row order of each world's codeword slice
        let mut scratch = vec![F::default(); std::cmp::max(local_col_size, row_length)];
        local_matrix
            .chunks_mut(sub_matrix_per_world_len)
            .for_each(|c| transpose_inplace(c, &mut scratch, local_col_size, row_length));
        drop(scratch);
    }

    /*
    Eventually, a final transpose each row lead to results of global column order

    p(0)                    p(1)                 ....
    *     *     *     *     *     *     *     *
    |    /|    /|    /|     |    /|    /|    /|
    *   / *   / *   / *     *   / *   / *   / *
    |  /  |  /  |  /  |     |  /  |  /  |  /  |
    * /   * /   * /   *     * /   * /   * /   *
    |/    |/    |/    |     |/    |/    |/    |
    *     *     *     *     *     *     *     *

     */

    // NOTE: transpose back into column order
    let mut scratch = vec![F::default(); std::cmp::max(global_col_size, row_length)];
    transpose_inplace(local_matrix, &mut scratch, row_length, global_col_size);
    drop(scratch);
}
