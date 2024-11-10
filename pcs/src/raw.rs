use std::{borrow::Borrow, marker::PhantomData};

use crate::PolynomialCommitmentScheme;
use arith::{Field, FieldSerde};
use polynomials::MultiLinearPoly;
use rand::RngCore;
use transcript::Transcript;

#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyType {}

impl FieldSerde for EmptyType {
    const SERIALIZED_SIZE: usize = 0;

    fn serialize_into<W: std::io::Write>(&self, _writer: W) -> arith::FieldSerdeResult<()> {
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(_reader: R) -> arith::FieldSerdeResult<Self> {
        Ok(Self {})
    }

    fn try_deserialize_from_ecc_format<R: std::io::Read>(
        _reader: R,
    ) -> arith::FieldSerdeResult<Self> {
        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub struct RawMLParams {
    pub n_vars: usize,
}

// Raw commitment for multi-linear polynomials
pub struct RawML<F: Field + FieldSerde, T: Transcript<F>> {
    _phantom_f: PhantomData<F>,
    _phantom_t: PhantomData<T>,
}

impl<F: Field + FieldSerde, T: Transcript<F>> PolynomialCommitmentScheme for RawML<F, T> {
    type PublicParams = RawMLParams;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;
    type Eval = F;

    type SRS = EmptyType;
    type ProverKey = EmptyType;
    type VerifierKey = EmptyType;

    type Commitment = Vec<F>;
    type CommitmentWithData = Vec<F>;
    type OpeningProof = EmptyType;

    type FiatShamirTranscript = T;

    fn gen_srs_for_testing(_rng: impl RngCore, _params: &Self::PublicParams) -> Self::SRS {
        Self::SRS::default()
    }

    fn commit(
        params: &Self::PublicParams,
        _proving_key: impl Borrow<Self::ProverKey>,
        poly: &Self::Poly,
    ) -> Self::Commitment {
        assert!(1 << params.n_vars == poly.coeffs.len());
        poly.coeffs.clone()
    }

    fn open(
        params: &Self::PublicParams,
        _proving_key: impl Borrow<Self::ProverKey>,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _transcript: &mut Self::FiatShamirTranscript,
    ) -> (Self::Eval, Self::OpeningProof) {
        assert!(1 << params.n_vars == poly.coeffs.len());
        (poly.evaluate_jolt(x), Self::OpeningProof::default())
    }

    fn verify(
        params: &Self::PublicParams,
        _verifying_key: &Self::VerifierKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: Self::Eval,
        _opening: &Self::OpeningProof,
        _transcript: &mut Self::FiatShamirTranscript,
    ) -> bool {
        assert!(1 << params.n_vars == commitment.len());
        let ml_poly = MultiLinearPoly::<F> {
            coeffs: commitment.clone(),
        };
        ml_poly.evaluate_jolt(x) == v
    }
}
