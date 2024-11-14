//! Orion polynomial commitment scheme prototype implementaiton.
//! Includes implementation for Orion Expander-Code.

use std::{marker::PhantomData, ops::Mul};

use arith::{Field, FieldSerde, SimdField};
use polynomials::MultiLinearPoly;
use transcript::Transcript;

use crate::PolynomialCommitmentScheme;

mod utils;
pub use utils::{OrionPCSError, OrionResult};

mod orion_code;
pub use orion_code::{OrionCodeParameter, ORION_CODE_PARAMETER_INSTANCE};

mod pcs_impl;
pub use pcs_impl::{
    OrionCommitment, OrionCommitmentWithData, OrionProof, OrionPublicParams,
    ORION_PCS_SOUNDNESS_BITS,
};

#[cfg(test)]
mod tests;

/***************************************************
 * POLYNOMIAL COMMITMENT TRAIT ALIGNMENT FOR ORION *
 ***************************************************/

pub struct OrionPCS<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>
where
    F: Field + FieldSerde,
    EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    IPPackF: SimdField<Scalar = F>,
    IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
    T: Transcript<EvalF>,
{
    _marker_f: PhantomData<F>,
    _marker_pack_f: PhantomData<ComPackF>,
    _marker_eval_f: PhantomData<EvalF>,
    _marker_pack_f0: PhantomData<IPPackF>,
    _marker_pack_eval_f: PhantomData<IPPackEvalF>,
    _marker_t: PhantomData<T>,
}

#[derive(Clone, Debug)]
pub struct OrionPCSSetup {
    pub num_vars: usize,
    pub code_parameter: OrionCodeParameter,
}

impl<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T> PolynomialCommitmentScheme
    for OrionPCS<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>
where
    F: Field + FieldSerde,
    EvalF: Field + FieldSerde + From<F> + Mul<F, Output = EvalF>,
    ComPackF: SimdField<Scalar = F>,
    IPPackF: SimdField<Scalar = F>,
    IPPackEvalF: SimdField<Scalar = EvalF> + Mul<IPPackF, Output = IPPackEvalF>,
    T: Transcript<EvalF>,
{
    type PublicParams = OrionPCSSetup;

    type Poly = MultiLinearPoly<F>;

    type EvalPoint = Vec<EvalF>;
    type Eval = EvalF;

    type SRS = OrionPublicParams;
    type ProverKey = Self::SRS;
    type VerifierKey = Self::SRS;

    type Commitment = OrionCommitment;
    type CommitmentWithData = OrionCommitmentWithData<F, ComPackF>;
    type OpeningProof = OrionProof<EvalF>;

    type FiatShamirTranscript = T;

    fn gen_srs_for_testing(rng: impl rand::RngCore, params: &Self::PublicParams) -> Self::SRS {
        OrionPublicParams::from_random::<F>(params.num_vars, params.code_parameter, rng)
    }

    fn commit(proving_key: &Self::ProverKey, poly: &Self::Poly) -> Self::CommitmentWithData {
        proving_key.commit(poly).unwrap()
    }

    fn open(
        proving_key: &Self::ProverKey,
        poly: &Self::Poly,
        opening_point: &Self::EvalPoint,
        commitment_with_data: &Self::CommitmentWithData,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> (Self::Eval, Self::OpeningProof) {
        proving_key.open::<F, EvalF, ComPackF, IPPackF, IPPackEvalF, T>(
            poly,
            commitment_with_data,
            opening_point,
            transcript,
        )
    }

    fn verify(
        verifying_key: &Self::VerifierKey,
        commitment: &Self::Commitment,
        opening_point: &Self::EvalPoint,
        evaluation: Self::Eval,
        opening_proof: &Self::OpeningProof,
        transcript: &mut Self::FiatShamirTranscript,
    ) -> bool {
        verifying_key.verify::<F, ComPackF, EvalF, IPPackF, IPPackEvalF, T>(
            commitment,
            opening_point,
            evaluation,
            opening_proof,
            transcript,
        )
    }
}
