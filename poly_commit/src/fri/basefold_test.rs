use arith::{ExtensionField, FFTField, Fr};
use ark_std::test_rng;
use babybear::{BabyBear, BabyBearExt3};
use gkr_engine::Transcript;
use gkr_hashers::Keccak256hasher;
use goldilocks::{Goldilocks, GoldilocksExt2};
use mersenne31::M31Ext6;
use polynomials::MultiLinearPoly;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;

use crate::fri::{
    basefold_impl::{fri_commit, fri_open, fri_verify, LOG_CODE_RATE},
    FRIScratchPad,
};

// TODO(HS) later move to pcs e2e tests

fn fri_pcs_e2e_helper<F, ChallengeF>(num_vars: usize)
where
    F: FFTField,
    ChallengeF: ExtensionField + From<F> + ExpSerde + FFTField,
{
    let mut rng = test_rng();

    let mle = MultiLinearPoly::<F>::random(num_vars, &mut rng);
    let mut scratch_pad = FRIScratchPad::default();

    let point: Vec<ChallengeF> = (0..num_vars)
        .map(|_| ChallengeF::random_unsafe(&mut rng))
        .collect();

    let mut fs_transcript_p = BytesHashTranscript::<ChallengeF, Keccak256hasher>::new();
    let mut fs_transcript_v = fs_transcript_p.clone();

    let com = fri_commit(&mle, LOG_CODE_RATE, &mut scratch_pad);
    let (eval, opening) = fri_open(&mle, &point, &mut fs_transcript_p, &scratch_pad);
    let verif = fri_verify::<F, ChallengeF>(&com, &point, eval, &opening, &mut fs_transcript_v);
    assert!(verif)
}

#[test]
fn test_fri_pcs_e2e() {
    fri_pcs_e2e_helper::<Fr, Fr>(16);
    fri_pcs_e2e_helper::<M31Ext6, M31Ext6>(16);
    fri_pcs_e2e_helper::<GoldilocksExt2, GoldilocksExt2>(16);
    fri_pcs_e2e_helper::<Goldilocks, GoldilocksExt2>(16);
    fri_pcs_e2e_helper::<BabyBearExt3, BabyBearExt3>(16);
    fri_pcs_e2e_helper::<BabyBear, BabyBearExt3>(16);
}
