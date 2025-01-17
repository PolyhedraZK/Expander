use arith::Field;
use gkr_field_config::GKRFieldConfig;
use mpi_config::MPIConfig;
use poly_commit::raw::RawExpanderGKR;
use poly_commit::{
    ExpanderGKRChallenge, PCSForExpanderGKR, PolynomialCommitmentScheme, StructuredReferenceString,
};
use polynomials::{MultilinearExtension, RefMultiLinearPoly};
use rand::thread_rng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
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
    mpi_config: &MPIConfig,
    n_local_vars: usize,
) {
    let mut rng = thread_rng();
    let params = P::gen_params(n_local_vars);

    let srs = P::gen_srs_for_testing(&params, mpi_config, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();

    let num_threads = rayon::current_num_threads();

    (0..num_threads).into_par_iter().for_each(|_| {
        let mut rng = thread_rng();
        let hypercube_basis = (0..(1 << n_local_vars))
            .map(|_| C::SimdCircuitField::random_unsafe(&mut rng))
            .collect();
        let poly = RefMultiLinearPoly::from_ref(&hypercube_basis);

        let xs = (0..2)
            .map(|_| ExpanderGKRChallenge::<C> {
                x: (0..n_local_vars)
                    .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                    .collect::<Vec<C::ChallengeField>>(),
                x_simd: (0..C::get_field_pack_size().trailing_zeros())
                    .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                    .collect::<Vec<C::ChallengeField>>(),
                x_mpi: (0..mpi_config.world_size().trailing_zeros())
                    .map(|_| C::ChallengeField::random_unsafe(&mut rng))
                    .collect::<Vec<C::ChallengeField>>(),
            })
            .collect::<Vec<ExpanderGKRChallenge<C>>>();

        let mut scratch_pad = P::init_scratch_pad(&params, mpi_config);

        let commitment = P::commit(&params, mpi_config, &proving_key, &poly, &mut scratch_pad);
        let mut transcript = T::new();

        // PCSForExpanderGKR does not require an evaluation value for the opening function
        // We use RawExpanderGKR as the golden standard for the evaluation value
        // Note this test will almost always pass for RawExpanderGKR, so make sure it is correct
        let start = mpi_config.current_size();
        let end = start + poly.serialized_size();

        poly.hypercube_basis_ref()
            .iter()
            .for_each(|f| mpi_config.append_local_field(f));
        let coeffs_gathered = mpi_config.read_all_field_flat(start, end);

        for xx in xs {
            let ExpanderGKRChallenge { x, x_simd, x_mpi } = &xx;
            let opening = P::open(
                &params,
                mpi_config,
                &proving_key,
                &poly,
                &xx,
                &mut transcript,
                &mut scratch_pad,
            );

            // this will always pass for RawExpanderGKR, so make sure it is correct
            let v = RawExpanderGKR::<C, T>::eval(&coeffs_gathered, &x, &x_simd, &x_mpi);

            assert!(P::verify(
                &params,
                mpi_config,
                &verification_key,
                &commitment,
                &xx,
                v,
                &mut transcript,
                &opening
            ));
        }
    });
}
