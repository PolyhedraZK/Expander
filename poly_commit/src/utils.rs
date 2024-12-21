use arith::FieldSerde;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use transcript::Transcript;

use crate::{PCSForExpanderGKR, StructuredReferenceString};

#[derive(Clone, Debug, Default)]
pub struct PCSEmptyType {}

impl FieldSerde for PCSEmptyType {
    const SERIALIZED_SIZE: usize = 0;

    fn serialize_into<W: std::io::Write>(&self, _writer: W) -> arith::FieldSerdeResult<()> {
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(_reader: R) -> arith::FieldSerdeResult<Self> {
        Ok(Self {})
    }
}

impl StructuredReferenceString for PCSEmptyType {
    type PKey = PCSEmptyType;
    type VKey = PCSEmptyType;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (Self {}, Self {})
    }
}

#[allow(clippy::type_complexity)]
pub fn expander_pcs_init_testing_only<
    FieldConfig: GKRFieldConfig,
    T: Transcript<FieldConfig::ChallengeField>,
    PCS: PCSForExpanderGKR<FieldConfig, T>,
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
