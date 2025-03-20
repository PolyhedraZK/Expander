use derivative::Derivative;
use halo2curves::{pairing::Engine, CurveAffine};
use serdes::ExpSerde;

use crate::StructuredReferenceString;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct KZGCommitment<E: Engine>(pub E::G1Affine)
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>;

/// Structured reference string for univariate KZG polynomial commitment scheme.
/// The univariate polynomial here is of coefficient form.
#[derive(Clone, Debug, PartialEq, Eq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct CoefFormUniKZGSRS<E: Engine>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    /// power of \tau times the generators of G1, yielding
    /// \tau^i over G1 with i ranging in \[ 0, 2^n - 1 \]
    pub powers_of_tau: Vec<E::G1Affine>,
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> StructuredReferenceString for CoefFormUniKZGSRS<E>
where
    <E as Engine>::G1Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    <E as Engine>::G2Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    type PKey = CoefFormUniKZGSRS<E>;
    type VKey = UniKZGVerifierParams<E>;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        let vk: Self::VKey = From::from(&self);
        (self, vk)
    }
}

/// Univariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UniKZGVerifierParams<E: Engine> {
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> From<&CoefFormUniKZGSRS<E>> for UniKZGVerifierParams<E>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    fn from(value: &CoefFormUniKZGSRS<E>) -> Self {
        Self {
            tau_g2: value.tau_g2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct CoefFormBiKZGLocalSRS<E: Engine>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    pub tau_x_srs: CoefFormUniKZGSRS<E>,
    pub tau_y_srs: CoefFormUniKZGSRS<E>,
}

/// Bivariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct BiKZGVerifierParam<E: Engine>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    /// tau_x over G2.
    pub tau_x_g2: E::G2Affine,
    /// tau_y over G2.
    pub tau_y_g2: E::G2Affine,
}

impl<E: Engine> From<&CoefFormBiKZGLocalSRS<E>> for BiKZGVerifierParam<E>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    fn from(srs: &CoefFormBiKZGLocalSRS<E>) -> Self {
        Self {
            tau_x_g2: srs.tau_x_srs.tau_g2,
            tau_y_g2: srs.tau_y_srs.tau_g2,
        }
    }
}

impl<E: Engine> StructuredReferenceString for CoefFormBiKZGLocalSRS<E>
where
    <E as Engine>::G1Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    <E as Engine>::G2Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    type PKey = CoefFormBiKZGLocalSRS<E>;
    type VKey = BiKZGVerifierParam<E>;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        let vk: Self::VKey = From::from(&self);
        (self, vk)
    }
}

impl<E: Engine> From<&BiKZGVerifierParam<E>> for UniKZGVerifierParams<E>
where
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2>,
{
    fn from(value: &BiKZGVerifierParam<E>) -> Self {
        Self {
            tau_g2: value.tau_x_g2,
        }
    }
}

/// Proof for Bi-KZG polynomial commitment scheme.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BiKZGProof<E: Engine> {
    pub quotient_x: E::G1Affine,
    pub quotient_y: E::G1Affine,
}
