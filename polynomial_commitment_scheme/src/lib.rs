mod traits;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use rand::thread_rng;
pub use traits::{
    ExpanderGKRChallenge, PCSForExpanderGKR, PolynomialCommitmentScheme, StructuredReferenceString,
};

#[allow(clippy::type_complexity)]
pub fn expander_pcs_init_unsafe<
    FieldConfig: GKRFieldConfig,
    PCS: PCSForExpanderGKR<FieldConfig>,
>(
    n_input_vars: usize,
    mpi_config: &MPIConfig,
) -> (
    PCS::Params,
    <PCS::SRS as StructuredReferenceString>::PKey,
    <PCS::SRS as StructuredReferenceString>::VKey,
    PCS::ScratchPad,
) {
    let mut rng = thread_rng();
    let pcs_params = <PCS as PCSForExpanderGKR<FieldConfig>>::gen_params(n_input_vars);
    let pcs_setup = <PCS as PCSForExpanderGKR<FieldConfig>>::gen_srs_for_testing(
        &pcs_params,
        mpi_config,
        &mut rng,
    );
    let (pcs_proving_key, pcs_verification_key) = pcs_setup.into_keys();
    let pcs_scratch =
        <PCS as PCSForExpanderGKR<FieldConfig>>::init_scratch_pad(&pcs_params, mpi_config);

    (
        pcs_params,
        pcs_proving_key,
        pcs_verification_key,
        pcs_scratch,
    )
}

mod utils;
use utils::PCSEmptyType;

pub mod raw;
