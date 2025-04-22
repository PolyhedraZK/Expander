use arith::{ExtensionField, FFTField, Fr};
use ark_std::test_rng;
use gkr_engine::Transcript;
use gkr_hashers::Keccak256hasher;
use polynomials::MultiLinearPoly;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;

use crate::fri::{
    basefold_impl::{fri_commit, fri_open},
    FRIScratchPad,
};

fn fri_pcs_e2e_helper<F, ChallengeF>(num_vars: usize)
where
    F: FFTField + ExtensionField,
    ChallengeF: ExtensionField<BaseField = F> + ExpSerde + FFTField,
{
    let mut rng = test_rng();

    let mle = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let mut scratch_pad = FRIScratchPad::default();

    let point: Vec<ChallengeF> = (0..num_vars)
        .map(|_| ChallengeF::random_unsafe(&mut rng))
        .collect();

    let mut fs_transcript_p = BytesHashTranscript::<ChallengeF, Keccak256hasher>::new();

    let _commitment = fri_commit(&mle.coeffs, &mut scratch_pad);

    fri_open(&mle, &point, &mut fs_transcript_p, &scratch_pad);
}

#[test]
fn test_fri_pcs_e2e() {
    fri_pcs_e2e_helper::<Fr, Fr>(19);
}
