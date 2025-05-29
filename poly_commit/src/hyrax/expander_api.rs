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

use super::hyrax_impl::{hyrax_batch_open, hyrax_batch_verify};

impl<G, C> ExpanderPCS<G, C::Scalar> for HyraxPCS<C>
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

    fn gen_params(n_input_vars: usize, _world_size: usize) -> Self::Params {
        n_input_vars
    }

    fn init_scratch_pad(_params: &Self::Params, _mpi_engine: &impl MPIEngine) -> Self::ScratchPad {}

    fn gen_srs_for_testing(
        params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        let mpi_vars = mpi_engine.world_size().ilog2() as usize;

        hyrax_setup(*params, mpi_vars, rng)
    }

    fn commit(
        _params: &Self::Params,
        mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<C::Scalar>,
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
        poly: &impl polynomials::MultilinearExtension<C::Scalar>,
        x: &ExpanderSingleVarChallenge<G>,
        _transcript: &mut impl Transcript,
        _scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        if mpi_engine.is_single_process() {
            let (_, open) = hyrax_open(proving_key, poly, &x.local_xs());
            return open.into();
        }

        let pedersen_len = proving_key.msm_len();
        let pedersen_vars = pedersen_len.ilog2() as usize;

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
        evals: <G as FieldEngine>::ChallengeField,
        _transcript: &mut impl Transcript,
        opening: &Self::Opening,
    ) -> bool {
        if x.r_mpi.is_empty() {
            return hyrax_verify(verifying_key, commitment, &x.local_xs(), evals, opening);
        }

        let pedersen_len = verifying_key.msm_len();
        let pedersen_vars = pedersen_len.ilog2() as usize;

        let local_vars = x.local_xs();
        let mut non_row_vars = local_vars[pedersen_vars..].to_vec();
        non_row_vars.extend_from_slice(&x.r_mpi);

        let eq_combination: Vec<C::Scalar> = EqPolynomial::build_eq_x_r(&non_row_vars);
        let row_comm = msm::best_multiexp(&eq_combination, &commitment.0);

        if pedersen_commit(verifying_key, &opening.0) != row_comm.into() {
            return false;
        }

        let mut scratch = vec![C::Scalar::default(); opening.0.len()];
        evals
            == RefMultiLinearPoly::from_ref(&opening.0)
                .evaluate_with_buffer(&local_vars[..pedersen_vars], &mut scratch)
    }

    /// Open a set of polynomials at a point.
    fn batch_open(
        _params: &Self::Params,
        _mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        mle_poly_list: &[impl MultilinearExtension<C::Scalar>],
        eval_point: &ExpanderSingleVarChallenge<G>,
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (Vec<C::Scalar>, Self::Opening) {
        hyrax_batch_open(
            proving_key,
            mle_poly_list,
            &eval_point.local_xs(),
            transcript,
        )
    }

    fn batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitments: &[Self::Commitment],
        x: &ExpanderSingleVarChallenge<G>,
        evals: &[<G as FieldEngine>::ChallengeField],
        opening: &Self::Opening,
        transcript: &mut impl Transcript,
    ) -> bool {
        hyrax_batch_verify(
            verifying_key,
            commitments,
            &x.local_xs(),
            evals,
            opening,
            transcript,
        )
    }
}
