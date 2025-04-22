use arith::ExtensionField;
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, MPIEngine, PolynomialCommitmentType,
    StructuredReferenceString, Transcript,
};
use halo2curves::{ff::PrimeField, msm, CurveAffine};
use polynomials::{
    EqPolynomial, MultilinearExtension, MutRefMultiLinearPoly, MutableMultilinearExtension,
    RefMultiLinearPoly,
};
use serdes::ExpSerde;

use crate::{
    hyrax::{
        hyrax_impl::{hyrax_commit, hyrax_open, hyrax_setup, hyrax_verify},
        pedersen::pedersen_commit,
    },
    HyraxCommitment, HyraxOpening, HyraxPCS, PedersenParams,
};

impl<G, C> ExpanderPCS<G> for HyraxPCS<C>
where
    G: FieldEngine<ChallengeField = C::Scalar, SimdCircuitField = C::Scalar>,
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    const NAME: &'static str = "HyraxPCSForExpanderGKR";

    const PCS_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::Hyrax;

    type Params = usize;
    type ScratchPad = ();

    type Commitment = HyraxCommitment<C>;
    type Opening = HyraxOpening<C>;
    type SRS = PedersenParams<C>;

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_engine: &impl MPIEngine) -> Self::ScratchPad {}

    fn gen_srs_for_testing(
        params: &Self::Params,
        _mpi_engine: &impl MPIEngine,
        rng: impl rand::RngCore,
    ) -> (Self::SRS, usize) {
        (hyrax_setup(*params, rng), *params)
    }

    fn commit(
        _params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as FieldEngine>::SimdCircuitField>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        let local_commit = hyrax_commit(proving_key, poly);

        if mpi_engine.is_single_process() {
            return local_commit.into();
        }

        let mut global_commit: Vec<C> = if mpi_engine.is_root() {
            vec![C::default(); mpi_engine.world_size() * local_commit.0.len()]
        } else {
            vec![]
        };

        mpi_engine.gather_vec(&local_commit.0, &mut global_commit);
        if !mpi_engine.is_root() {
            return None;
        }

        HyraxCommitment(global_commit).into()
    }

    fn open(
        _params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as FieldEngine>::SimdCircuitField>,
        x: &ExpanderSingleVarChallenge<G>,
        _transcript: &mut impl Transcript<G::ChallengeField>,
        _scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        if mpi_engine.is_single_process() {
            let (_, open) = hyrax_open(proving_key, poly, &x.local_xs());
            return open.into();
        }

        let local_num_vars = poly.num_vars();
        let pedersen_vars = (local_num_vars + 1) / 2;
        let pedersen_len = 1usize << pedersen_vars;
        assert_eq!(pedersen_len, proving_key.bases.len());

        let local_vars = x.local_xs();
        let mut local_basis = poly.hypercube_basis();
        let mut local_mle = MutRefMultiLinearPoly::from_ref(&mut local_basis);
        local_mle.fix_variables(&local_vars[pedersen_vars..]);

        let eq_mpi_vars = EqPolynomial::build_eq_x_r(&x.r_mpi);
        let combined_coeffs = mpi_engine.coef_combine_vec(&local_basis, &eq_mpi_vars);

        if !mpi_engine.is_root() {
            return None;
        }

        HyraxOpening(combined_coeffs).into()
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderSingleVarChallenge<G>,
        v: <G as FieldEngine>::ChallengeField,
        _transcript: &mut impl Transcript<G::ChallengeField>,
        opening: &Self::Opening,
    ) -> bool {
        if x.r_mpi.is_empty() {
            return hyrax_verify(verifying_key, commitment, &x.local_xs(), v, opening);
        }

        let local_num_vars = x.local_xs().len();
        let pedersen_vars = (local_num_vars + 1) / 2;
        let pedersen_len = 1usize << pedersen_vars;
        assert_eq!(pedersen_len, verifying_key.bases.len());

        let local_vars = x.local_xs();
        let mut non_row_vars = local_vars[pedersen_vars..].to_vec();
        non_row_vars.extend_from_slice(&x.r_mpi);

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
