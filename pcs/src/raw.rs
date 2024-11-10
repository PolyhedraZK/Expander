use std::marker::PhantomData;

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
pub struct RawMultilinearPCSPublicParams {
    pub n_vars: usize,
}

// Raw commitment for multi-linear polynomials
pub struct RawMultilinearPCS<F: Field + FieldSerde, T: Transcript<F>> {
    _phantom_f: PhantomData<F>,
    _phantom_t: PhantomData<T>,
}

impl<F: Field + FieldSerde, T: Transcript<F>> PolynomialCommitmentScheme
    for RawMultilinearPCS<F, T>
{
    type PublicParams = RawMultilinearPCSPublicParams;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<F>;
    type Eval = F;

    type SRS = Self::PublicParams;
    type ProverKey = Self::PublicParams;
    type VerifierKey = Self::PublicParams;

    type Commitment = Vec<F>;
    type CommitmentWithData = Vec<F>;
    type OpeningProof = EmptyType;

    type FiatShamirTranscript = T;

    fn gen_srs_for_testing(_rng: impl RngCore, params: &Self::PublicParams) -> Self::SRS {
        params.clone()
    }

    fn commit(proving_key: &Self::ProverKey, poly: &Self::Poly) -> Self::Commitment {
        assert!(1 << proving_key.n_vars == poly.coeffs.len());
        poly.coeffs.clone()
    }

    fn open(
        prover_key: &Self::ProverKey,
        poly: &Self::Poly,
        x: &Self::EvalPoint,
        _commitment_with_data: &Self::Commitment,
        _transcript: &mut Self::FiatShamirTranscript,
    ) -> (Self::Eval, Self::OpeningProof) {
        assert!(1 << prover_key.n_vars == poly.coeffs.len());
        (poly.evaluate_jolt(x), Self::OpeningProof::default())
    }

    fn verify(
        verifier_key: &Self::VerifierKey,
        commitment: &Self::Commitment,
        x: &Self::EvalPoint,
        v: Self::Eval,
        _opening: &Self::OpeningProof,
        _transcript: &mut Self::FiatShamirTranscript,
    ) -> bool {
        assert!(1 << verifier_key.n_vars == commitment.len());
        let ml_poly = MultiLinearPoly::<F> {
            coeffs: commitment.clone(),
        };
        ml_poly.evaluate_jolt(x) == v
    }
}
