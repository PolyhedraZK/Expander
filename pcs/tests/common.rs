use arith::{Field, FieldSerde};
use pcs::PolynomialCommitmentScheme;
use rand::RngCore;
use transcript::Transcript;

pub fn test_pcs<F: Field + FieldSerde, P: PolynomialCommitmentScheme>(
    params: &P::PublicParams,
    poly: &P::Poly,
    opening_points: &P::EvalPoint,
    mut rng: impl RngCore,
) {
    let srs = P::gen_srs_for_testing(&mut rng, params);
    let prover_key = srs.clone().into();
    let verifier_key = srs.clone().into();

    let commitment_with_data = P::commit(&prover_key, poly);
    let commitment = commitment_with_data.clone().into();

    {
        let mut transcript = P::FiatShamirTranscript::new();
        let mut transcript_copy = transcript.clone();

        let (v, opening) = P::open(&prover_key, poly, opening_points, &mut transcript);

        assert!(P::verify(
            &verifier_key,
            &commitment,
            opening_points,
            v,
            &opening,
            &mut transcript_copy
        ));
    }
}
