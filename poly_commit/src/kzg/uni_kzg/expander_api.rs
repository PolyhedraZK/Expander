use arith::ExtensionField;
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, MPIEngine, PolynomialCommitmentType,
    StructuredReferenceString, Transcript,
};
use halo2curves::{
    ff::PrimeField,
    pairing::{Engine, MultiMillerLoop},
    CurveAffine,
};
use polynomials::{MultiLinearPoly, MultilinearExtension};
use serdes::ExpSerde;

use crate::{traits::BatchOpening, *};

impl<G, E> ExpanderPCS<G, E::Fr> for HyperUniKZGPCS<E>
where
    G: FieldEngine<ChallengeField = E::Fr, SimdCircuitField = E::Fr>,
    E: Engine + MultiMillerLoop,
    E::Fr: ExtensionField + PrimeField,
    E::G1Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    const NAME: &'static str = "HyperUniKZGForExpander";

    const PCS_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::KZG;

    type Commitment = UniKZGCommitment<E>;
    type Opening = HyperUniKZGOpening<E>;
    type Params = usize;
    type SRS = CoefFormUniKZGSRS<E>;
    type ScratchPad = ();
    type BatchOpening = BatchOpening<E::Fr, Self>;

    fn init_scratch_pad(_params: &Self::Params, _mpi_engine: &impl MPIEngine) -> Self::ScratchPad {}

    fn gen_params(n_input_vars: usize, _world_size: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs(
        params: &Self::Params,
        _mpi_engine: &impl MPIEngine,
        rng: impl rand::RngCore,
    ) -> Self::SRS {
        let size = 1 << *params;
        generate_coef_form_uni_kzg_srs_for_testing(size, rng)
    }

    fn commit(
        _params: &Self::Params,
        _mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<E::Fr>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        let commitment = coeff_form_uni_kzg_commit(proving_key, poly.hypercube_basis_ref());
        Some(UniKZGCommitment(commitment))
    }

    fn open(
        _params: &Self::Params,
        _mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<E::Fr>,
        x: &ExpanderSingleVarChallenge<G>,
        transcript: &mut impl Transcript,
        _scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
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
        params: &Self::Params,
        _mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        polys: &[impl MultilinearExtension<E::Fr>],
        x: &[ExpanderSingleVarChallenge<G>],
        scratch_pad: &Self::ScratchPad,
        transcript: &mut impl Transcript,
    ) -> (Vec<E::Fr>, Self::BatchOpening) {
        let points: Vec<Vec<E::Fr>> = x.iter().map(|p| p.local_xs()).collect();

        multiple_points_batch_open_impl(proving_key, polys, points.as_ref(), transcript)
    }

    fn multi_points_batch_verify(
        _params: &Self::Params,
        verifying_key: &<Self::SRS as StructuredReferenceString>::VKey,
        commitments: &[impl AsRef<Self::Commitment>],
        x: &[ExpanderSingleVarChallenge<G>],
        evals: &[E::Fr],
        batch_opening: &Self::BatchOpening,
        transcript: &mut impl Transcript,
    ) -> bool {
        let points: Vec<Vec<E::Fr>> = x.iter().map(|p| p.local_xs()).collect();

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
