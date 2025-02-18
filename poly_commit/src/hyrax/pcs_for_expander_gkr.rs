use arith::{ExtensionField, FieldSerde};
use gkr_field_config::GKRFieldConfig;
use halo2curves::{ff::PrimeField, msm, CurveAffine};
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension, RefMultiLinearPoly};
use transcript::Transcript;

use crate::{
    hyrax::{
        hyrax_impl::{hyrax_commit, hyrax_open, hyrax_setup, hyrax_verify},
        inner_prod_argument::{pedersen_ipa_prove, pedersen_ipa_verify},
    },
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
        mpi_config: &mpi_config::MPIConfig,
        proving_key: &<Self::SRS as crate::StructuredReferenceString>::PKey,
        poly: &impl polynomials::MultilinearExtension<<G as GKRFieldConfig>::SimdCircuitField>,
        scratch_pad: &mut Self::ScratchPad,
    ) -> Self::Commitment {
        let local_commit = hyrax_commit(proving_key, poly, scratch_pad);

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
        transcript: &mut T, // add transcript here to allow interactive arguments
        scratch_pad: &Self::ScratchPad,
    ) -> Self::Opening {
        if mpi_config.is_single_process() {
            let (_, open) = hyrax_open(proving_key, poly, &x.local_xs(), scratch_pad, transcript);
            return open;
        }

        let local_num_vars = poly.num_vars();
        let pedersen_vars = (local_num_vars + 1) / 2;
        let pedersen_len = 1usize << pedersen_vars;
        assert_eq!(pedersen_len, proving_key.bases.len());

        let local_vars = x.local_xs();
        let mut local_mle = MultiLinearPoly::new(poly.hypercube_basis());
        local_vars[pedersen_vars..]
            .iter()
            .rev()
            .for_each(|e| local_mle.fix_top_variable(*e));

        let eq_mpi_vars = EqPolynomial::build_eq_x_r(&x.x_mpi);
        let combined_coeffs = mpi_config.coef_combine_vec(&local_mle.coeffs, &eq_mpi_vars);
        let combined_com_randomness = mpi_config.coef_combine_vec(scratch_pad, &eq_mpi_vars);

        let mut buffer = vec![C::Scalar::default(); scratch_pad.len()];
        let row_eqs = EqPolynomial::build_eq_x_r(&local_vars[..pedersen_vars]);

        if mpi_config.is_root() {
            let final_com_randomness = RefMultiLinearPoly::from_ref(&combined_com_randomness)
                .evaluate_with_buffer(&local_vars[pedersen_vars..], &mut buffer);
            return pedersen_ipa_prove(
                proving_key,
                &combined_coeffs,
                &row_eqs,
                final_com_randomness,
                transcript,
            );
        }

        let final_com_randomness = RefMultiLinearPoly::from_ref(scratch_pad)
            .evaluate_with_buffer(&local_vars[pedersen_vars..], &mut buffer);
        pedersen_ipa_prove(
            proving_key,
            &local_mle.coeffs,
            &row_eqs,
            final_com_randomness,
            transcript,
        )
    }

    fn verify(
        #[allow(unused)] params: &Self::Params,
        mpi_config: &mpi_config::MPIConfig,
        verifying_key: &<Self::SRS as crate::StructuredReferenceString>::VKey,
        commitment: &Self::Commitment,
        x: &crate::ExpanderGKRChallenge<G>,
        v: <G as GKRFieldConfig>::ChallengeField,
        transcript: &mut T, // add transcript here to allow interactive arguments
        opening: &Self::Opening,
    ) -> bool {
        if mpi_config.is_single_process() || !mpi_config.is_root() {
            return hyrax_verify(
                verifying_key,
                commitment,
                &x.local_xs(),
                v,
                opening,
                transcript,
            );
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

        let row_eqs = EqPolynomial::build_eq_x_r(&local_vars[..pedersen_vars]);
        pedersen_ipa_verify(
            verifying_key,
            row_comm.into(),
            opening,
            &row_eqs,
            v,
            transcript,
        )
    }
}
