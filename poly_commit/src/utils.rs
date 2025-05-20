use arith::Field;
use ark_std::test_rng;
use gkr_engine::{ExpanderPCS, FieldEngine, MPIConfig, StructuredReferenceString};

#[allow(clippy::type_complexity)]
pub fn expander_pcs_init_testing_only<
    FieldConfig: FieldEngine,
    PCSPolyField: Field,
    PCS: ExpanderPCS<FieldConfig, PCSPolyField>,
>(
    n_input_vars: usize,
    mpi_config: &MPIConfig,
) -> (
    PCS::Params,
    <PCS::SRS as StructuredReferenceString>::PKey,
    <PCS::SRS as StructuredReferenceString>::VKey,
    PCS::ScratchPad,
) {
    let mut rng = test_rng();

    let mut pcs_params = <PCS as ExpanderPCS<FieldConfig, PCSPolyField>>::gen_params(n_input_vars);
    let (pcs_setup, calibrated_num_local_simd_vars) = <PCS as ExpanderPCS<
        FieldConfig,
        PCSPolyField,
    >>::gen_srs_for_testing(
        &pcs_params, mpi_config, &mut rng
    );

    if n_input_vars < calibrated_num_local_simd_vars {
        eprintln!(
            "{} over {} has minimum supported local vars {}, but input poly has vars {}.",
            PCS::NAME,
            FieldConfig::SimdCircuitField::NAME,
            calibrated_num_local_simd_vars,
            n_input_vars,
        );
        pcs_params = <PCS as ExpanderPCS<FieldConfig, PCSPolyField>>::gen_params(
            calibrated_num_local_simd_vars,
        );
    }

    let (pcs_proving_key, pcs_verification_key) = pcs_setup.into_keys();
    let pcs_scratch =
        <PCS as ExpanderPCS<FieldConfig, PCSPolyField>>::init_scratch_pad(&pcs_params, mpi_config);

    (
        pcs_params,
        pcs_proving_key,
        pcs_verification_key,
        pcs_scratch,
    )
}
