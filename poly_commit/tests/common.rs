use arith::Field;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use poly_commit::raw::RawExpanderGKR;
use poly_commit::{
    ExpanderGKRChallenge, PCSForExpanderGKR, PolynomialCommitmentScheme, StructuredReferenceString,
};
use polynomials::MultiLinearPoly;
use rand::thread_rng;
use transcript::Transcript;

pub fn test_pcs<F: Field, P: PolynomialCommitmentScheme<F>>(
    params: &P::Params,
    poly: &P::Poly,
    xs: &[P::EvalPoint],
) {
    let mut rng = thread_rng();
    let srs = P::gen_srs_for_testing(params, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();
    let mut scratch_pad = P::init_scratch_pad(params);

    let commitment = P::commit(params, &proving_key, poly, &mut scratch_pad);

    for x in xs {
        let (v, opening) = P::open(params, &proving_key, poly, x, &mut scratch_pad);
        assert!(P::verify(
            params,
            &verification_key,
            &commitment,
            x,
            v,
            &opening
        ));
    }
}

pub fn test_gkr_pcs<
    C: GKRFieldConfig,
    T: Transcript<C::ChallengeField>,
    P: PCSForExpanderGKR<C, T>,
>(
    params: &P::Params,
    mpi_config: &MPIConfig,
    transcript: &mut T,
    poly: &MultiLinearPoly<C::SimdCircuitField>,
    xs: &[ExpanderGKRChallenge<C>],
) {
    let mut rng = thread_rng();
    let srs = P::gen_srs_for_testing(params, mpi_config, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();
    let mut scratch_pad = P::init_scratch_pad(params, mpi_config);

    let commitment = P::commit(params, mpi_config, &proving_key, poly, &mut scratch_pad);

    // PCSForExpanderGKR does not require an evaluation value for the opening function
    // We use RawExpanderGKR as the golden standard for the evaluation value
    // Note this test will almost always pass for RawExpanderGKR, so make sure it is correct
    let mut coeffs_gathered = if mpi_config.is_root() {
        vec![C::SimdCircuitField::ZERO; poly.coeffs.len() * mpi_config.world_size()]
    } else {
        vec![]
    };
    mpi_config.gather_vec(&poly.coeffs, &mut coeffs_gathered);

    for xx in xs {
        let ExpanderGKRChallenge { x, x_simd, x_mpi } = xx;

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
            let v = RawExpanderGKR::<C, T>::eval(&coeffs_gathered, x, x_simd, x_mpi);

            transcript.lock_proof();
            assert!(P::verify(
                params,
                mpi_config,
                &verification_key,
                &commitment,
                xx,
                v,
                transcript,
                &opening
            ));
            transcript.unlock_proof();
        }
    }
}
