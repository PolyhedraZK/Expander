// use babybear::BabyBear;
use p3_baby_bear::PackedBabyBearAVX512 as BabyBearx16;
use rand::RngCore;

use crate::PolynomialCommitmentScheme;

pub struct BaseFoldPCS;

impl PolynomialCommitmentScheme for BaseFoldPCS {
    type ProverParam = ();
    type VerifierParam = ();
    type SRS = ();
    type Polynomial = ();
    type Point = ();
    type Evaluation = BabyBearx16;
    type Commitment = ();
    type Proof = ();
    type BatchProof = ();

    fn gen_srs_for_testing(
        _rng: impl RngCore,
        _supported_n: usize,
        _supported_m: usize,
    ) -> Self::SRS {
        unimplemented!()
    }

    fn commit(
        _prover_param: impl std::borrow::Borrow<Self::ProverParam>,
        _poly: &Self::Polynomial,
    ) -> Self::Commitment {
        unimplemented!()
    }

    fn open(
        _prover_param: impl std::borrow::Borrow<Self::ProverParam>,
        _polynomial: &Self::Polynomial,
        _point: &Self::Point,
    ) -> (Self::Proof, Self::Evaluation) {
        unimplemented!()
    }

    fn verify(
        _verifier_param: &Self::VerifierParam,
        _commitment: &Self::Commitment,
        _point: &Self::Point,
        _value: &Self::Evaluation,
        _proof: &Self::Proof,
    ) -> bool {
        unimplemented!()
    }
}
