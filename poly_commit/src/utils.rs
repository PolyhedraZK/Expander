use gkr_engine::{ExpanderPCS, FieldEngine, MPIConfig, StructuredReferenceString, Transcript};

#[allow(clippy::type_complexity)]
pub fn expander_pcs_init_testing_only<
    FieldConfig: FieldEngine,
    T: Transcript<FieldConfig::ChallengeField>,
    PCS: ExpanderPCS<FieldConfig>,
>(
    n_input_vars: usize,
    mpi_config: &MPIConfig,
    mut rng: impl rand::RngCore,
) -> (
    PCS::Params,
    <PCS::SRS as StructuredReferenceString>::PKey,
    <PCS::SRS as StructuredReferenceString>::VKey,
    PCS::ScratchPad,
) {
    let pcs_params = <PCS as ExpanderPCS<FieldConfig>>::gen_params(n_input_vars);
    let pcs_setup =
        <PCS as ExpanderPCS<FieldConfig>>::gen_srs_for_testing(&pcs_params, mpi_config, &mut rng);
    let (pcs_proving_key, pcs_verification_key) = pcs_setup.into_keys();
    let pcs_scratch = <PCS as ExpanderPCS<FieldConfig>>::init_scratch_pad(&pcs_params, mpi_config);

    (
        pcs_params,
        pcs_proving_key,
        pcs_verification_key,
        pcs_scratch,
    )
}
