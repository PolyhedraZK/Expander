use arith::ExtensionField;
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, MPIEngine, PolynomialCommitmentType,
    StructuredReferenceString, Transcript,
};
use halo2curves::{
    ff::PrimeField,
    group::prime::PrimeCurveAffine,
    pairing::{Engine, MultiMillerLoop},
    CurveAffine,
};
use serdes::ExpSerde;

use crate::*;

impl<G, E> ExpanderPCS<G> for HyperKZGPCS<E>
where
    G: FieldEngine<ChallengeField = E::Fr, SimdCircuitField = E::Fr>,
    E: Engine + MultiMillerLoop,
    E::Fr: ExtensionField + PrimeField,
    E::G1Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::G2Affine: ExpSerde + Default + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    const NAME: &'static str = "HyperKZGPCSForExpander";

    const PCS_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::KZG;

    type Commitment = KZGCommitment<E>;
    type Opening = HyperBiKZGOpening<E>;
    type Params = usize;
    type SRS = CoefFormBiKZGLocalSRS<E>;
    type ScratchPad = ();

    fn init_scratch_pad(_params: &Self::Params, _mpi_engine: &impl MPIEngine) -> Self::ScratchPad {}

    fn gen_params(n_input_vars: usize) -> Self::Params {
        n_input_vars
    }

    fn gen_srs_for_testing(
        params: &Self::Params,
        // mpi_engine: &impl MPIEngine,
        rng: impl rand::RngCore,
    ) -> (Self::SRS, usize) {
        let local_num_vars = if *params == 0 { 1 } else { *params };

        let x_degree_po2 = 1 << local_num_vars;
        let y_degree_po2 = 1; //mpi_engine.world_size();
        let rank = 0; // mpi_engine.world_rank();

        let srs =
            generate_coef_form_bi_kzg_local_srs_for_testing(x_degree_po2, y_degree_po2, rank, rng);
        (srs, local_num_vars)
    }

    fn commit(
        _params: &Self::Params,
        // mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as FieldEngine>::SimdCircuitField>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        let local_commitment =
            coeff_form_uni_kzg_commit(&proving_key.tau_x_srs, poly.hypercube_basis_ref());

        // if mpi_engine.is_single_process() {
        return KZGCommitment(local_commitment).into();
        // }

        // let local_g1 = local_commitment.to_curve();
        // let mut root_gathering_commits: Vec<E::G1> = vec![local_g1; mpi_engine.world_size()];
        // mpi_engine.gather_vec(&[local_g1], &mut root_gathering_commits);

        // if !mpi_engine.is_root() {
        //     return None;
        // }

        // let final_commit = root_gathering_commits.iter().sum::<E::G1>().into();

        // KZGCommitment(final_commit).into()
    }

    fn open(
        _params: &Self::Params,
        // mpi_engine: &impl MPIEngine,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as FieldEngine>::SimdCircuitField>,
        x: &ExpanderSingleVarChallenge<G>,
        transcript: &mut impl Transcript,
        _scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        coeff_form_hyper_bikzg_open(
            proving_key,
            // mpi_engine,
            poly,
            &x.local_xs(),
            // &x.r_mpi,
            transcript,
        )
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
        coeff_form_hyper_bikzg_verify(
            verifying_key,
            &x.local_xs(),
            &x.r_mpi,
            v,
            commitment.0,
            opening,
            transcript,
        )
    }
}
