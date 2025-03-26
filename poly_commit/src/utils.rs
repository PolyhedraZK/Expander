use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use transcript::Transcript;

use crate::{PCSForExpanderGKR, StructuredReferenceString};

impl StructuredReferenceString for () {
    type PKey = ();
    type VKey = ();

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        ((), ())
    }
}

const PCS_TESTING_SEED_U64: u64 = 114514;

#[allow(clippy::type_complexity)]
pub fn expander_pcs_init_testing_only<
    FieldConfig: GKRFieldConfig,
    T: Transcript<FieldConfig::ChallengeField>,
    PCS: PCSForExpanderGKR<FieldConfig, T>,
>(
    n_input_vars: usize,
    mpi_config: &MPIConfig,
) -> (
    PCS::Params,
    <PCS::SRS as StructuredReferenceString>::PKey,
    <PCS::SRS as StructuredReferenceString>::VKey,
    PCS::ScratchPad,
) {
    let mut rng = ChaCha12Rng::seed_from_u64(PCS_TESTING_SEED_U64);

    let pcs_params = <PCS as PCSForExpanderGKR<FieldConfig, T>>::gen_params(n_input_vars);
    let pcs_setup = <PCS as PCSForExpanderGKR<FieldConfig, T>>::gen_srs_for_testing(
        &pcs_params,
        mpi_config,
        &mut rng,
    );
    let (pcs_proving_key, pcs_verification_key) = pcs_setup.into_keys();
    let pcs_scratch =
        <PCS as PCSForExpanderGKR<FieldConfig, T>>::init_scratch_pad(&pcs_params, mpi_config);

    (
        pcs_params,
        pcs_proving_key,
        pcs_verification_key,
        pcs_scratch,
    )
}
