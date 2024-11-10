use arith::{Field, FieldSerde};
use pcs::PCS;
use rand::thread_rng;
use transcript::Transcript;

pub fn test_pcs<F: Field + FieldSerde, P: PCS>(
    pcs: &mut P,
    params: &P::Params,
    poly: &P::Poly,
    xs: &[P::EvalPoint],
) {
    let mut rng = thread_rng();
    let srs = pcs.gen_srs_for_testing(&mut rng, params);
    let proving_key = srs.clone().into();
    let verification_key = srs.clone().into();

    let commitment = pcs.commit(params, &proving_key, poly);

    for x in xs {
        let mut _transcript = P::FiatShamirTranscript::new();
        let mut _transcript_copy = _transcript.clone();

        let (v, opening) = pcs.open(params, &proving_key, poly, x, &mut _transcript);
        assert!(P::verify(
            params,
            &verification_key,
            &commitment,
            x,
            v,
            &opening,
            &mut _transcript_copy
        ));
    }
}
