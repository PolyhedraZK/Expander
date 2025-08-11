use arith::ExtensionField;
use ark_ec::pairing::Pairing;
use ark_std::rand::RngCore;
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, PolynomialCommitmentType,
    StructuredReferenceString, Transcript,
};
use polynomials::MultilinearExtension;
use serdes::ExpSerde;

use crate::{
    traits::BatchOpening,
    utils::{
        lift_expander_challenge_to_n_vars, lift_poly_and_expander_challenge_to_n_vars,
        lift_poly_to_n_vars,
    },
    *,
};

impl<G, E> ExpanderPCS<G> for HyperUniKZGPCS<E>
where
    G: FieldEngine<ChallengeField = E::ScalarField, SimdCircuitField = E::ScalarField>,
    E: Pairing,
    E::ScalarField: ExtensionField,
    E::G1Affine: ExpSerde + Default,
    E::G2Affine: ExpSerde + Default,
{
    const NAME: &'static str = "HyperUniKZGForExpander";

    const PCS_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::KZG;

    type Commitment = UniKZGCommitment<E>;
    type Opening = HyperUniKZGOpening<E>;
    type Params = usize;
    type SRS = CoefFormUniKZGSRS<E>;
    type ScratchPad = ();
    type BatchOpening = BatchOpening<E::ScalarField, Self>;

    fn init_scratch_pad(_params: &Self::Params,) -> Self::ScratchPad {}

    fn gen_params(n_input_vars: usize, world_size: usize) -> Self::Params {
        assert_eq!(
            world_size, 1,
            "HyperUniKZGPCS is not parallelized, world size must be 1"
        );
        std::cmp::max(n_input_vars, Self::MINIMUM_SUPPORTED_NUM_VARS)
    }

    fn gen_srs(
        params: &Self::Params,
       
        rng: impl RngCore,
    ) -> Self::SRS {
        assert!(
            *params >= Self::MINIMUM_SUPPORTED_NUM_VARS,
            "params must be at least {}",
            Self::MINIMUM_SUPPORTED_NUM_VARS
        );
        let size = 1 << *params;
        generate_coef_form_uni_kzg_srs_for_testing(size, rng)
    }

    fn commit(
        params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<E::ScalarField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        if poly.num_vars() < Self::MINIMUM_SUPPORTED_NUM_VARS {
            assert_eq!(*params, Self::MINIMUM_SUPPORTED_NUM_VARS);
            let poly = lift_poly_to_n_vars(poly, *params);
            return <Self as ExpanderPCS<G>>::commit(
                params,
                proving_key,
                &poly,
                scratch_pad,
            );
        }

        let commitment = coeff_form_uni_kzg_commit(proving_key, poly.hypercube_basis_ref());
        Some(UniKZGCommitment(commitment))
    }

    fn open(
        params: &Self::Params,
        
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<E::ScalarField>,
        x: &ExpanderSingleVarChallenge<G>,
        transcript: &mut impl Transcript,
        scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        if poly.num_vars() < Self::MINIMUM_SUPPORTED_NUM_VARS {
            assert_eq!(*params, Self::MINIMUM_SUPPORTED_NUM_VARS);
            let (poly, x) = lift_poly_and_expander_challenge_to_n_vars(poly, x, *params);
            return <Self as ExpanderPCS<G>>::open(
                params,
                proving_key,
                &poly,
                &x,
                transcript,
                scratch_pad,
            );
        }

        let (_eval, open) = coeff_form_uni_hyperkzg_open(
            proving_key,
            poly.hypercube_basis_ref(),
            &x.local_xs(),
            transcript,
        );

        Some(open)
    }

    fn verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &ExpanderSingleVarChallenge<G>,
        v: <G as FieldEngine>::ChallengeField,
        transcript: &mut impl Transcript,
        opening: &Self::Opening,
    ) -> bool {
        if x.rz.len() < Self::MINIMUM_SUPPORTED_NUM_VARS {
            let x = lift_expander_challenge_to_n_vars(x, Self::MINIMUM_SUPPORTED_NUM_VARS);
            return <Self as ExpanderPCS<G>>::verify(
                _params,
                verifying_key,
                commitment,
                &x,
                v,
                transcript,
                opening,
            );
        }

        coeff_form_uni_hyperkzg_verify(
            verifying_key,
            commitment.0,
            &x.local_xs(),
            v,
            opening,
            transcript,
        )
    }

    /// Open a set of polynomials at a point.
    fn multi_points_batch_open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        polys: &[impl MultilinearExtension<E::ScalarField>],
        x: &[ExpanderSingleVarChallenge<G>],
        _scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (Vec<E::ScalarField>, Self::BatchOpening) {
        let points: Vec<Vec<E::ScalarField>> = x.iter().map(|p| p.local_xs()).collect();

        multiple_points_batch_open_impl(proving_key, polys, points.as_ref(), transcript)
    }

    fn multi_points_batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitments: &[impl AsRef<Self::Commitment>],
        x: &[ExpanderSingleVarChallenge<G>],
        evals: &[E::ScalarField],
        batch_opening: &Self::BatchOpening,
        transcript: &mut impl Transcript,
    ) -> bool {
        let points: Vec<Vec<E::ScalarField>> = x.iter().map(|p| p.local_xs()).collect();

        multiple_points_batch_verify_impl(
            verifying_key,
            commitments,
            points.as_ref(),
            evals,
            batch_opening,
            transcript,
        )
    }
}
