use ark_ec::pairing::Pairing;
use derivative::Derivative;
use gkr_engine::StructuredReferenceString;
use serdes::{ExpSerde, SerdeResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct UniKZGCommitment<E: Pairing>(pub E::G1Affine);

impl<E: Pairing> AsRef<UniKZGCommitment<E>> for UniKZGCommitment<E> {
    fn as_ref(&self) -> &UniKZGCommitment<E> {
        self
    }
}

// Derive macros does not work for associated types
impl<E: Pairing> ExpSerde for UniKZGCommitment<E>
where
    E::G1Affine: ExpSerde,
{
    fn serialize_into<W: std::io::Write>(&self, writer: W) -> SerdeResult<()> {
        self.0.serialize_into(writer)
    }

    fn deserialize_from<R: std::io::Read>(reader: R) -> SerdeResult<Self> {
        Ok(Self(<E::G1Affine as ExpSerde>::deserialize_from(reader)?))
    }
}

/// Structured reference string for univariate KZG polynomial commitment scheme.
/// The univariate polynomial here is of coefficient form.
#[derive(Clone, Debug, PartialEq, Eq, Derivative, ExpSerde)]
#[derivative(Default(bound = ""))]
pub struct CoefFormUniKZGSRS<E: Pairing>
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    /// power of \tau times the generators of G1, yielding
    /// \tau^i over G1 with i ranging in \[ 0, 2^n - 1 \]
    pub powers_of_tau: Vec<E::G1Affine>,
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Pairing> StructuredReferenceString for CoefFormUniKZGSRS<E>
where
    <E as Pairing>::G1Affine: ExpSerde,
    <E as Pairing>::G2Affine: ExpSerde,
{
    type PKey = CoefFormUniKZGSRS<E>;
    type VKey = UniKZGVerifierParams<E>;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        let vk: Self::VKey = From::from(&self);
        (self, vk)
    }
}

/// Univariate KZG PCS verifier's params.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ExpSerde)]
pub struct UniKZGVerifierParams<E: Pairing>
where
    E::G2Affine: ExpSerde,
{
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Pairing> From<&CoefFormUniKZGSRS<E>> for UniKZGVerifierParams<E>
where
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
{
    fn from(value: &CoefFormUniKZGSRS<E>) -> Self {
        Self {
            tau_g2: value.tau_g2,
        }
    }
}
