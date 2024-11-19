use arith::{Field, FieldSerde};
use gkr_field_config::GKRFieldConfig;
use polynomial_commitment_scheme::raw::RawExpanderGKR;
use polynomial_commitment_scheme::{ExpanderGKRChallenge, PCSForExpanderGKR, PolynomialCommitmentScheme, StructuredReferenceString};
use polynomials::MultiLinearPoly;
use rand::thread_rng;

pub fn test_pcs<F: Field + FieldSerde, P: PolynomialCommitmentScheme<F>>(
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

pub fn test_gkr_pcs<C: GKRFieldConfig, P: PCSForExpanderGKR<C>>(
    params: &P::Params,
    poly: &MultiLinearPoly<C::SimdCircuitField>,
    xs: &[ExpanderGKRChallenge<C>],
) {
    let mut rng = thread_rng();
    let srs = P::gen_srs_for_testing(params, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();
    let mut scratch_pad = P::init_scratch_pad(params);

    let commitment = P::commit(params, &proving_key, poly, &mut scratch_pad);

    for xx in xs {
        let ExpanderGKRChallenge { x, x_simd, x_mpi } = xx;
        let v = RawExpanderGKR::<C>::eval(&poly.coeffs, x, x_simd, x_mpi); // this will always pass for RawExpanderGKR, so make sure it is correct
        let opening = P::open(params, &proving_key, poly, xx, &mut scratch_pad);
        assert!(P::verify(
            params,
            &verification_key,
            &commitment,
            xx,
            v,
            &opening
        ));
    }
}
