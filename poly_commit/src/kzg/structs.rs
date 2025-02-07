use arith::FieldSerde;
use halo2curves::{pairing::Engine, CurveAffine};

use crate::StructuredReferenceString;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KZGCommitment<E: Engine>(pub E::G1Affine);

impl<E: Engine> Default for KZGCommitment<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    fn default() -> Self {
        Self(E::G1Affine::default())
    }
}

impl<E: Engine> FieldSerde for KZGCommitment<E> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        todo!()
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        todo!()
    }
}

/// Structured reference string for univariate KZG polynomial commitment scheme.
/// The univariate polynomial here is of coefficient form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoefFormUniKZGSRS<E: Engine> {
    /// power of \tau times the generators of G1, yielding
    /// \tau^i over G1 with i ranging in \[ 0, 2^n - 1 \]
    pub powers_of_tau: Vec<E::G1Affine>,
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> Default for CoefFormUniKZGSRS<E>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    fn default() -> Self {
        Self {
            powers_of_tau: Vec::default(),
            tau_g2: E::G2Affine::default(),
        }
    }
}

impl<E: Engine> FieldSerde for CoefFormUniKZGSRS<E> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        todo!()
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        todo!()
    }
}

impl<E: Engine> StructuredReferenceString for CoefFormUniKZGSRS<E> {
    type PKey = CoefFormUniKZGSRS<E>;
    type VKey = UniKZGVerifierParams<E>;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (self.clone(), From::from(&self))
    }
}

/// Univariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UniKZGVerifierParams<E: Engine> {
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> FieldSerde for UniKZGVerifierParams<E> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        todo!()
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        todo!()
    }
}

impl<E: Engine> From<&CoefFormUniKZGSRS<E>> for UniKZGVerifierParams<E> {
    fn from(value: &CoefFormUniKZGSRS<E>) -> Self {
        Self {
            tau_g2: value.tau_g2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HyperKZGOpening<E: Engine> {
    pub folded_oracle_commitments: Vec<E::G1Affine>,
    pub f_beta2: E::Fr,
    pub evals_at_beta: Vec<E::Fr>,
    pub evals_at_neg_beta: Vec<E::Fr>,
    pub beta_commitment: E::G1Affine,
    pub tau_vanishing_commitment: E::G1Affine,
}

impl<E: Engine> Default for HyperKZGOpening<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    fn default() -> Self {
        todo!()
    }
}

impl<E: Engine> FieldSerde for HyperKZGOpening<E> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> arith::FieldSerdeResult<()> {
        todo!()
    }

    fn deserialize_from<R: std::io::Read>(mut reader: R) -> arith::FieldSerdeResult<Self> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoefFormBiKZGLocalSRS<E: Engine> {
    pub tau_x_srs: CoefFormUniKZGSRS<E>,
    pub tau_y_srs: CoefFormUniKZGSRS<E>,
}

/// Bivariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct BiKZGVerifierParam<E: Engine> {
    /// tau_0 over G2.
    pub tau_x_g2: E::G2Affine,
    /// tau_y over G2.
    pub tau_y_g2: E::G2Affine,
}

impl<E: Engine> From<&CoefFormBiKZGLocalSRS<E>> for BiKZGVerifierParam<E> {
    fn from(srs: &CoefFormBiKZGLocalSRS<E>) -> Self {
        Self {
            tau_x_g2: srs.tau_x_srs.tau_g2,
            tau_y_g2: srs.tau_y_srs.tau_g2,
        }
    }
}

/// Proof for Bi-KZG polynomial commitment scheme.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BiKZGProof<E: Engine> {
    pub quotient_x: E::G1Affine,
    pub quotient_y: E::G1Affine,
}
