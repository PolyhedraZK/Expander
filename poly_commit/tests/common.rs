use arith::{ExtensionField, Field};
use ark_std::test_rng;
use gkr_engine::{
    ExpanderPCS, ExpanderSingleVarChallenge, FieldEngine, MPIConfig, MPIEngine,
    StructuredReferenceString, Transcript,
};
use poly_commit::PolynomialCommitmentScheme;
use polynomials::{MultiLinearPoly, MultilinearExtension};
use rand::thread_rng;

pub fn test_pcs<F: ExtensionField, T: Transcript, P: PolynomialCommitmentScheme<F>>(
    params: &P::Params,
    poly: &P::Poly,
    xs: &[P::EvalPoint],
) {
    let mut rng = thread_rng();
    // NOTE(HS) we assume that the polynomials we pass in are of sufficient length.
    let (srs, _) = P::gen_srs_for_testing(params, &mut rng);
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

#[allow(dead_code)]
pub fn test_batching<F, T, P>()
where
    F: ExtensionField,
    T: Transcript,
    P: PolynomialCommitmentScheme<F, Params = usize, EvalPoint = Vec<F>, Poly = MultiLinearPoly<F>>,
{
    let mut rng = thread_rng();

    for num_vars in 2..10 {
        // NOTE(HS) we assume that the polynomials we pass in are of sufficient length.
        let (srs, _) = P::gen_srs_for_testing(&num_vars, &mut rng);

        let mut scratch_pad = P::init_scratch_pad(&num_vars);

        let (proving_key, verification_key) = srs.into_keys();

        for num_poly in [1, 2, 10, 100] {
            let polys = (0..num_poly)
                .map(|_| MultiLinearPoly::<F>::random(num_vars, &mut rng))
                .collect::<Vec<_>>();
            let commitments = polys
                .iter()
                .map(|poly| P::commit(&num_vars, &proving_key, poly, &mut scratch_pad))
                .collect::<Vec<_>>();

            // open all polys at a single point
            let x = (0..num_vars)
                .map(|_| F::random_unsafe(&mut rng))
                .collect::<Vec<_>>();

            let mut transcript = T::new();

            let (values, batch_opening) = P::batch_open(
                &num_vars,
                &proving_key,
                &polys,
                x.as_ref(),
                &mut scratch_pad,
                &mut transcript,
            );

            let mut transcript = T::new();

            assert!(P::batch_verify(
                &num_vars,
                &verification_key,
                &commitments,
                x.as_ref(),
                &values,
                &batch_opening,
                &mut transcript
            ))
        }
    }
}


#[allow(dead_code)]
pub fn test_pcs_for_expander_gkr<
    C: FieldEngine,
    T: Transcript,
    P: ExpanderPCS<C, C::SimdCircuitField>,
>(
    params: &P::Params,
    mpi_config: &MPIConfig,
    transcript: &mut T,
    poly: &impl MultilinearExtension<C::SimdCircuitField>,
    xs: &[ExpanderSingleVarChallenge<C>],
) {
    let mut rng = test_rng();
    // NOTE(HS) we assume that the polynomials we pass in are of sufficient length.
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
            let v = C::single_core_eval_circuit_vals_at_expander_challenge(&coeffs_gathered, xx);

            transcript.lock_proof();
            assert!(P::verify(
                params,
                &verification_key,
                &commitment.clone().unwrap(),
                xx,
                v,
                &mut transcript_cloned,
                &opening.unwrap()
            ));
            transcript.unlock_proof();
        }
    }
}

pub fn test_batching_for_expander_gkr<C, T, P>()
where
    C: FieldEngine,
    T: Transcript,
    P: ExpanderPCS<C, C::SimdCircuitField, Params = usize>,
{
    let mut rng = test_rng();
    let mpi_config = MPIConfig::prover_new();
    for num_vars in 2..10 {
        // NOTE(HS) we assume that the polynomials we pass in are of sufficient length.
        let srs = P::gen_srs_for_testing(&num_vars, &mpi_config, &mut rng);
        let (proving_key, verification_key) = srs.into_keys();
        let mut scratch_pad = P::init_scratch_pad(&num_vars, &mpi_config);

        for num_poly in [1, 2, 10, 100] {
            let polys = (0..num_poly)
                .map(|_| MultiLinearPoly::<C::SimdCircuitField>::random(num_vars, &mut rng))
                .collect::<Vec<_>>();

            let commitments = polys
                .iter()
                .map(|poly| {
                    P::commit(&num_vars, &mpi_config, &proving_key, poly, &mut scratch_pad).unwrap()
                })
                .collect::<Vec<_>>();

            // open all polys at a single point
            let challenge_point = ExpanderSingleVarChallenge::<C> {
                r_mpi: Vec::new(),
                r_simd: Vec::new(),
                rz: (0..num_vars)
                    .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                    .collect(),
            };

            let mut transcript = T::new();

            let (eval_list, opening) = P::batch_open(
                &num_vars,
                &mpi_config,
                &proving_key,
                &polys,
                &challenge_point,
                &mut scratch_pad,
                &mut transcript,
            );

            let mut transcript = T::new();

            assert!(P::batch_verify(
                &num_vars,
                &verification_key,
                &commitments,
                &challenge_point,
                &eval_list,
                &opening,
                &mut transcript,
            ));
        }
    }
}
