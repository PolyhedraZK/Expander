use arith::{ExtensionField, Field};
use ark_std::test_rng;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use poly_commit::{
    ExpanderGKRChallenge, PCSForExpanderGKR, PolynomialCommitmentScheme, StructuredReferenceString,
};
use polynomials::{MultiLinearPolyExpander, MultilinearExtension};
use rand::thread_rng;
use transcript::Transcript;

pub fn test_pcs<F: ExtensionField, T: Transcript<F>, P: PolynomialCommitmentScheme<F, T>>(
    params: &P::Params,
    poly: &P::Poly,
    xs: &[P::EvalPoint],
) {
    let mut rng = thread_rng();
    let srs = P::gen_srs_for_testing(params, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();
    let mut transcript = T::new();
    let mut scratch_pad = P::init_scratch_pad(params);

    let commitment = P::commit(params, &proving_key, poly, &mut scratch_pad);

    for x in xs {
        let mut transcript_cloned = transcript.clone();
        let (v, opening) = P::open(
            params,
            &proving_key,
            poly,
            x,
            &mut scratch_pad,
            &mut transcript,
        );
        assert!(P::verify(
            params,
            &verification_key,
            &commitment,
            x,
            v,
            &opening,
            &mut transcript_cloned
        ));
    }
}

pub fn test_pcs_for_expander_gkr<
    C: GKRFieldConfig,
    T: Transcript<C::ChallengeField>,
    P: PCSForExpanderGKR<C, T>,
>(
    params: &P::Params,
    mpi_config: &MPIConfig,
    transcript: &mut T,
    poly: &impl MultilinearExtension<C::SimdCircuitField>,
    xs: &[ExpanderGKRChallenge<C>],
) {
    let mut rng = test_rng();
    let srs = P::gen_srs_for_testing(params, mpi_config, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();
    let mut scratch_pad = P::init_scratch_pad(params, mpi_config);

    let commitment = P::commit(params, mpi_config, &proving_key, poly, &mut scratch_pad);

    // PCSForExpanderGKR does not require an evaluation value for the opening function
    // We use RawExpanderGKR as the golden standard for the evaluation value
    // Note this test will almost always pass for RawExpanderGKR, so make sure it is correct
    let mut coeffs_gathered = if mpi_config.is_root() {
        vec![C::SimdCircuitField::ZERO; poly.hypercube_size() * mpi_config.world_size()]
    } else {
        vec![]
    };
    mpi_config.gather_vec(poly.hypercube_basis_ref(), &mut coeffs_gathered);

    for xx in xs {
        let ExpanderGKRChallenge { x, x_simd, x_mpi } = xx;
        let mut transcript_cloned = transcript.clone();

        transcript.lock_proof();
        let opening = P::open(
            params,
            mpi_config,
            &proving_key,
            poly,
            xx,
            transcript,
            &mut scratch_pad,
        );
        transcript.unlock_proof();

        if mpi_config.is_root() {
            // this will always pass for RawExpanderGKR, so make sure it is correct
            let v =
                MultiLinearPolyExpander::<C>::single_core_eval_circuit_vals_at_expander_challenge(
                    &coeffs_gathered,
                    x,
                    x_simd,
                    x_mpi,
                );

            transcript.lock_proof();
            assert!(P::verify(
                params,
                mpi_config,
                &verification_key,
                &commitment,
                xx,
                v,
                &mut transcript_cloned,
                &opening
            ));
            transcript.unlock_proof();
        }
    }
}
