use arith::{ExtensionField, FieldSerde};
use gkr_field_config::GKRFieldConfig;
use halo2curves::{ff::PrimeField, msm, CurveAffine};
use polynomials::{
    EqPolynomial, MultilinearExtension, MutRefMultiLinearPoly, MutableMultilinearExtension,
    RefMultiLinearPoly,
};
use transcript::Transcript;

use crate::{
    hyrax::{
        hyrax_impl::{hyrax_commit, hyrax_open, hyrax_setup, hyrax_verify},
        pedersen::pedersen_commit,
    },
    HyraxCommitment, HyraxOpening, HyraxPCS, PCSForExpanderGKR, PedersenParams,
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
    type ScratchPad = ();

    type Commitment = HyraxCommitment<C>;
    type Opening = HyraxOpening<C>;
    type SRS = PedersenParams<C>;

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn init_scratch_pad(
        #[allow(unused)] params: &Self::Params,
        #[allow(unused)] mpi_config: &mpi_config::MPIConfig,
    ) -> Self::ScratchPad {
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
        mpi_config: &mpi_config::MPIConfig,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as GKRFieldConfig>::SimdCircuitField>,
        #[allow(unused)] scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let local_commit = hyrax_commit(proving_key, poly);

        if mpi_config.is_single_process() {
            return local_commit;
        }

        let mut global_commit: Vec<C> = if mpi_config.is_root() {
            vec![C::default(); mpi_config.world_size() * local_commit.0.len()]
        } else {
            vec![]
        };

        mpi_config.gather_vec(&local_commit.0, &mut global_commit);
        if mpi_config.is_root() {
            return HyraxCommitment(global_commit);
        }

        local_commit
    }

    fn open(
        #[allow(unused)] params: &Self::Params,
        mpi_config: &mpi_config::MPIConfig,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as GKRFieldConfig>::SimdCircuitField>,
        x: &crate::ExpanderGKRChallenge<G>,
        #[allow(unused)] transcript: &mut T, // add transcript here to allow interactive arguments
        #[allow(unused)] scratch_pad: &Self::ScratchPad,
    ) -> Self::Opening {
        if mpi_config.is_single_process() {
            let (_, open) = hyrax_open(proving_key, poly, &x.local_xs());
            return open;
        }

        let local_num_vars = poly.num_vars();
        let pedersen_vars = (local_num_vars + 1) / 2;
        let pedersen_len = 1usize << pedersen_vars;
        assert_eq!(pedersen_len, proving_key.bases.len());

        let local_vars = x.local_xs();
        let mut local_basis = poly.hypercube_basis();
        let mut local_mle = MutRefMultiLinearPoly::from_ref(&mut local_basis);
        local_mle.fix_variables(&local_vars[pedersen_vars..]);

        let eq_mpi_vars = EqPolynomial::build_eq_x_r(&x.x_mpi);
        let combined_coeffs = mpi_config.coef_combine_vec(&local_basis, &eq_mpi_vars);

        if mpi_config.is_root() {
            return HyraxOpening(combined_coeffs);
        }

        HyraxOpening(local_basis)
    }

    fn verify(
        #[allow(unused)] params: &Self::Params,
        mpi_config: &mpi_config::MPIConfig,
        verifying_key: &<Self::SRS as crate::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &crate::ExpanderGKRChallenge<G>,
        v: <G as GKRFieldConfig>::ChallengeField,
        #[allow(unused)] transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        if mpi_config.is_single_process() || !mpi_config.is_root() {
            return hyrax_verify(verifying_key, commitment, &x.local_xs(), v, opening);
        }

        let local_num_vars = x.local_xs().len();
        let pedersen_vars = (local_num_vars + 1) / 2;
        let pedersen_len = 1usize << pedersen_vars;
        assert_eq!(pedersen_len, verifying_key.bases.len());

        let local_vars = x.local_xs();
        let mut non_row_vars = local_vars[pedersen_vars..].to_vec();
        non_row_vars.extend_from_slice(&x.x_mpi);

        let eq_combination: Vec<C::Scalar> = EqPolynomial::build_eq_x_r(&non_row_vars);
        let mut row_comm = C::Curve::default();
        msm::multiexp_serial(&eq_combination, &commitment.0, &mut row_comm);

        if pedersen_commit(verifying_key, &opening.0) != row_comm.into() {
            return false;
        }

        let mut scratch = vec![C::Scalar::default(); opening.0.len()];
        v == RefMultiLinearPoly::from_ref(&opening.0)
            .evaluate_with_buffer(&local_vars[..pedersen_vars], &mut scratch)
    }
}
