use pcs::PolynomialCommitmentScheme;
use rand::RngCore;
use transcript::Transcript;

pub fn test_pcs_e2e<P: PolynomialCommitmentScheme>(
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

        let (eval, opening_proof) = P::open(
            &prover_key,
            poly,
            opening_points,
            &commitment_with_data,
            &mut transcript,
        );

        assert!(P::verify(
            &verifier_key,
            &commitment,
            opening_points,
            eval,
            &opening_proof,
            &mut transcript_copy
        ));
    }
}
