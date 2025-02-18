use arith::{ExtensionField, FieldSerde};
use gkr_field_config::GKRFieldConfig;
use halo2curves::{ff::PrimeField, CurveAffine};
use transcript::Transcript;

use crate::{
    hyrax::hyrax_impl::{hyrax_commit, hyrax_open, hyrax_setup},
    HyraxCommitment, HyraxPCS, PCSForExpanderGKR, PedersenIPAProof, PedersenParams,
};

impl<G, C, T> PCSForExpanderGKR<G, T> for HyraxPCS<C, T>
where
    G: GKRFieldConfig<ChallengeField = C::Scalar, SimdCircuitField = C::Scalar>,
    C: CurveAffine + FieldSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
    T: Transcript<G::ChallengeField>,
{
    const NAME: &'static str = "HyraxPCSForExpanderGKR";

    type Params = usize;
    type ScratchPad = Vec<G::ChallengeField>;

    type Commitment = HyraxCommitment<C>;
    type Opening = PedersenIPAProof<C>;
    type SRS = PedersenParams<C>;

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn init_scratch_pad(
        #[allow(unused)] params: &Self::Params,
        #[allow(unused)] mpi_config: &mpi_config::MPIConfig,
    ) -> Self::ScratchPad {
        Vec::new()
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        #[allow(unused)] mpi_config: &mpi_config::MPIConfig,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        hyrax_setup(*params, rng)
    }

    fn commit(
        #[allow(unused)] params: &Self::Params,
        // TODO(HS) MPI changes update
        #[allow(unused)] mpi_config: &mpi_config::MPIConfig,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as GKRFieldConfig>::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let _local_commit = hyrax_commit(proving_key, poly, scratch_pad);

        todo!()
    }

    fn open(
        #[allow(unused)] params: &Self::Params,
        // TODO(HS) MPI changes update
        #[allow(unused)] mpi_config: &mpi_config::MPIConfig,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as GKRFieldConfig>::SimdCircuitField>,
        x: &crate::ExpanderGKRChallenge<G>,
        transcript: &mut T, // add transcript here to allow interactive arguments
        scratch_pad: &Self::ScratchPad,
    ) -> Self::Opening {
        let (_local_opening, _local_eval) =
            hyrax_open(proving_key, poly, &x.local_xs(), scratch_pad, transcript);

        todo!()
    }

    #[allow(unused)]
    fn verify(
        #[allow(unused)] params: &Self::Params,
        // TODO(HS) MPI changes update
        #[allow(unused)] mpi_config: &mpi_config::MPIConfig,
        verifying_key: &<Self::SRS as crate::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &crate::ExpanderGKRChallenge<G>,
        v: <G as GKRFieldConfig>::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        todo!()
    }
}
