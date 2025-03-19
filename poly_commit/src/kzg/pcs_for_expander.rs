use arith::ExtensionField;
use gkr_field_config::GKRFieldConfig;
use halo2curves::{
    ff::PrimeField,
    group::prime::PrimeCurveAffine,
    pairing::{Engine, MultiMillerLoop},
    CurveAffine,
};
use serdes::ExpSerde;
use transcript::Transcript;

use crate::*;

impl<G, E, T> PCSForExpanderGKR<G, T> for HyperKZGPCS<E, T>
where
    G: GKRFieldConfig<ChallengeField = E::Fr, SimdCircuitField = E::Fr>,
    E: Engine + MultiMillerLoop,
    E::Fr: ExtensionField + PrimeField,
    E::G1Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
    T: Transcript<G::ChallengeField>,
{
    const NAME: &'static str = "HyperKZGPCSForExpander";

    type Commitment = KZGCommitment<E>;
    type Opening = HyperBiKZGOpening<E>;
    type Params = usize;
    type SRS = CoefFormBiKZGLocalSRS<E>;
    type ScratchPad = ();

    fn init_scratch_pad(
        #[allow(unused)] params: &Self::Params,
        #[allow(unused)] mpi_config: &mpi_config::MPIConfig,
    ) -> Self::ScratchPad {
    }

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_config: &mpi_config::MPIConfig,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        let x_degree_po2 = 1 << params;
        let y_degree_po2 = mpi_config.world_size();
        let rank = mpi_config.world_rank();

        generate_coef_form_bi_kzg_local_srs_for_testing(x_degree_po2, y_degree_po2, rank, rng)
    }

    fn commit(
        #[allow(unused)] params: &Self::Params,
        mpi_config: &mpi_config::MPIConfig,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as GKRFieldConfig>::SimdCircuitField>,
        #[allow(unused)] scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        let local_commitment =
            coeff_form_uni_kzg_commit(&proving_key.tau_x_srs, poly.hypercube_basis_ref());

        if mpi_config.is_single_process() {
            return KZGCommitment(local_commitment).into();
        }

        let mut root_gathering_commits: Vec<E::G1Affine> =
            vec![E::G1Affine::default(); mpi_config.world_size()];
        mpi_config.gather_vec(&vec![local_commitment], &mut root_gathering_commits);

        if !mpi_config.is_root() {
            return None;
        }

        let final_commit = root_gathering_commits
            .iter()
            .map(|c| c.to_curve())
            .sum::<E::G1>()
            .into();

        KZGCommitment(final_commit).into()
    }

    fn open(
        #[allow(unused)] params: &Self::Params,
        mpi_config: &mpi_config::MPIConfig,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as GKRFieldConfig>::SimdCircuitField>,
        x: &crate::ExpanderGKRChallenge<G>,
        transcript: &mut T, // add transcript here to allow interactive arguments
        #[allow(unused)] scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        coeff_form_hyper_bikzg_open(
            proving_key,
            mpi_config,
            poly,
            &x.local_xs(),
            &x.x_mpi,
            transcript,
        )
    }

    fn verify(
        #[allow(unused)] params: &Self::Params,
        verifying_key: &<Self::SRS as crate::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &crate::ExpanderGKRChallenge<G>,
        v: <G as GKRFieldConfig>::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        coeff_form_hyper_bikzg_verify(
            verifying_key,
            &x.local_xs(),
            &x.x_mpi,
            v,
            commitment.0,
            opening,
            transcript,
        )
    }
}
