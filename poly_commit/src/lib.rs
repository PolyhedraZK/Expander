mod traits;
use gkr_field_config::GKRFieldConfig;
use communicator::{MPICommunicator, ExpanderComm};
use rand::thread_rng;
pub use traits::{
    ExpanderGKRChallenge, PCSForExpanderGKR, PolynomialCommitmentScheme, StructuredReferenceString,
};

#[allow(clippy::type_complexity)]
pub fn expander_pcs_init_testing_only<
    FieldConfig: GKRFieldConfig,
    T: Transcript<FieldConfig::ChallengeField>,
    PCS: PCSForExpanderGKR<FieldConfig, T>,
>(
    n_input_vars: usize,
    mpi_comm: &MPICommunicator,
) -> (
    PCS::Params,
    <PCS::SRS as StructuredReferenceString>::PKey,
    <PCS::SRS as StructuredReferenceString>::VKey,
    PCS::ScratchPad,
) {
    let mut rng = thread_rng();
    let pcs_params = <PCS as PCSForExpanderGKR<FieldConfig, T>>::gen_params(n_input_vars);
    let pcs_setup = <PCS as PCSForExpanderGKR<FieldConfig, T>>::gen_srs_for_testing(
        &pcs_params,
        mpi_comm,
        &mut rng,
    );
    let (pcs_proving_key, pcs_verification_key) = pcs_setup.into_keys();
    let pcs_scratch =
        <PCS as PCSForExpanderGKR<FieldConfig, T>>::init_scratch_pad(&pcs_params, mpi_comm);

    (
        pcs_params,
        pcs_proving_key,
        pcs_verification_key,
        pcs_scratch,
    )
}

mod utils;
use transcript::Transcript;
use utils::PCSEmptyType;

pub mod raw;
