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
    utils::{
        lift_expander_challenge_to_n_vars, lift_poly_and_expander_challenge_to_n_vars,
        lift_poly_to_n_vars,
    },
    *,
};

impl<G, E> ExpanderPCS<G> for HyperBiKZGPCS<E>
where
    G: FieldEngine<ChallengeField = E::ScalarField, SimdCircuitField = E::ScalarField>,
    E: Pairing,
    E::ScalarField: ExtensionField,
    E::G1Affine: ExpSerde + Default,
    E::G2Affine: ExpSerde + Default,
{
    const NAME: &'static str = "HyperBiKZGForExpander";

    const PCS_TYPE: PolynomialCommitmentType = PolynomialCommitmentType::KZG;

    type Commitment = BiKZGCommitment<E>;
    type Opening = HyperBiKZGOpening<E>;
    type Params = usize;
    type SRS = CoefFormBiKZGLocalSRS<E>;
    type ScratchPad = ();
    type BatchOpening = ();

    fn init_scratch_pad(_params: &Self::Params) -> Self::ScratchPad {}

    fn gen_params(n_input_vars: usize, _world_size: usize) -> Self::Params {
        std::cmp::max(n_input_vars, Self::MINIMUM_SUPPORTED_NUM_VARS)
    }

    fn gen_srs(params: &Self::Params, rng: impl RngCore) -> Self::SRS {
        unimplemented!()
        // let local_num_vars = *params;

        // let x_degree_po2 = 1 << local_num_vars;
        // let y_degree_po2 = mpi_engine.world_size();
        // let rank = mpi_engine.world_rank();

        // generate_coef_form_bi_kzg_local_srs_for_testing(x_degree_po2, y_degree_po2, rank, rng)
    }

    fn commit(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<E::ScalarField>,
        _scratch_pad: &mut Self::ScratchPad,
    ) -> Option<Self::Commitment> {
        unimplemented!()
        // // The minimum supported number of variables is 1.
        // // If the polynomial has no variables, we lift it to a polynomial with 1 variable.
        // if poly.num_vars() < Self::MINIMUM_SUPPORTED_NUM_VARS {
        //     let poly = lift_poly_to_n_vars(poly, Self::MINIMUM_SUPPORTED_NUM_VARS);
        //     return <Self as ExpanderPCS<G>>::commit(
        //         _params,
        //         proving_key,
        //         &poly,
        //         _scratch_pad,
        //     );
        // };

        // let local_commitment =
        //     coeff_form_uni_kzg_commit(&proving_key.tau_x_srs, poly.hypercube_basis_ref());

        // if mpi_engine.is_single_process() {
        //     return BiKZGCommitment(local_commitment).into();
        // }

        // let local_g1: E::G1 = local_commitment.into();
        // let mut root_gathering_commits: Vec<E::G1> = vec![local_g1; mpi_engine.world_size()];
        // mpi_engine.gather_vec(&[local_g1], &mut root_gathering_commits);

        // if !mpi_engine.is_root() {
        //     return None;
        // }

        // let final_commit = root_gathering_commits.iter().sum::<E::G1>().into();

        // BiKZGCommitment(final_commit).into()
    }

    fn open(
        _params: &Self::Params,
        proving_key: &<Self::SRS as StructuredReferenceString>::PKey,
        poly: &impl MultilinearExtension<E::ScalarField>,
        x: &ExpanderSingleVarChallenge<G>,
        transcript: &mut impl Transcript,
        _scratch_pad: &Self::ScratchPad,
    ) -> Option<Self::Opening> {
        unimplemented!()
        // if poly.num_vars() < Self::MINIMUM_SUPPORTED_NUM_VARS {
        //     let (poly, x) = lift_poly_and_expander_challenge_to_n_vars(
        //         poly,
        //         x,
        //         Self::MINIMUM_SUPPORTED_NUM_VARS,
        //     );
        //     return <Self as ExpanderPCS<G>>::open(
        //         _params,
        //         proving_key,
        //         &poly,
        //         &x,
        //         transcript,
        //         _scratch_pad,
        //     );
        // };

        // coeff_form_hyper_bikzg_open(
        //     proving_key,
        //     poly,
        //     &x.local_xs(),
        //     &x.r_mpi,
        //     transcript,
        // )
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
        };

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
