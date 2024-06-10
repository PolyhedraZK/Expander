use std::{borrow::Borrow, marker::PhantomData};

use halo2curves::pairing::Engine;
use rand::Rng;

use crate::{
    pcs::PolynomialCommitmentScheme, BiKZGCommitment, BiKZGProof, BiKZGProverParam, BiKZGSRS,
    BiKZGVerifierParam,
};

pub struct BiKZG<E: Engine> {
    _engine: PhantomData<E>,
}

impl<E: Engine> PolynomialCommitmentScheme for BiKZG<E> {
    type SRS = BiKZGSRS<E>;
    type ProverParam = BiKZGProverParam<E>;
    type VerifierParam = BiKZGVerifierParam<E>;
    type Polynomial = Vec<E::Fr>;
    type Commitment = BiKZGCommitment<E>;
    type Proof = BiKZGProof<E>;
    type Evaluation = E::Fr;
    type Point = E::Fr;
    type BatchProof = Vec<Self::Proof>;

    fn gen_srs_for_testing<R: Rng>(rng: &mut R, supported_size: usize) -> Self::SRS {
        unimplemented!()
    }

    fn trim(
        srs: impl Borrow<Self::SRS>,
        supported_degree: Option<usize>,
        supported_num_vars: Option<usize>,
    ) -> (Self::ProverParam, Self::VerifierParam) {
        unimplemented!()
    }

    fn commit(
        prover_param: impl Borrow<Self::ProverParam>,
        poly: &Self::Polynomial,
    ) -> Self::Commitment {
        unimplemented!()
    }

    fn open(
        prover_param: impl Borrow<Self::ProverParam>,
        polynomial: &Self::Polynomial,
        point: &Self::Point,
    ) -> (Self::Proof, Self::Evaluation) {
        unimplemented!()
    }

    fn verify(
        verifier_param: &Self::VerifierParam,
        commitment: &Self::Commitment,
        point: &Self::Point,
        value: &Self::Evaluation,
        proof: &Self::Proof,
    ) -> bool {
        unimplemented!()
    }

    fn multi_open(
        _prover_param: impl Borrow<Self::ProverParam>,
        _polynomials: &[Self::Polynomial],
        _points: &[Self::Point],
        _evals: &[Self::Evaluation],
        // _transcript: &mut IOPTranscript<E::ScalarField>,
    ) -> Self::BatchProof {
        unimplemented!()
    }

    fn batch_verify(
        _verifier_param: &Self::VerifierParam,
        _commitments: &[Self::Commitment],
        _points: &[Self::Point],
        _batch_proof: &Self::BatchProof,
    ) -> bool {
        unimplemented!()
    }
}
