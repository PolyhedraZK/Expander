use std::io::{self, Read, Write};

use halo2curves::{pairing::Engine, serde::SerdeObject};

/// Structured reference string for Bi-KZG polynomial commitment scheme.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BiKZGSRS<E: Engine> {
    /// (g_1^{\tau_0^i\tau_1^j})_{i\in [0,N], j\in [0, M]} = \\
    /// (
    ///  g_1, g_1^{\tau_0}, g_1^{\tau_0^2}, ..., g_1^{\tau_0^N},
    ///  g_1^{\tau_1}, g_1^{\tau_0\tau_1}, g_1^{\tau_0^2\tau_1}, ..., g_1^{\tau_0^N\tau_1},
    ///  ..., g_1^{\tau_0^N\tau_1^M}
    /// )
    pub powers_of_g: Vec<E::G1Affine>,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// \beta times the above generator of G2.
    pub beta_h: E::G2Affine,
}

/// `BiKZGProverParam` is used to generate a proof
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct BiKZGProverParam<E: Engine> {
    /// Parameters
    pub powers_of_g: Vec<E::G1Affine>,
}

/// `UnivariateVerifierParam` is used to check evaluation proofs for a given
/// commitment.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct BiKZGVerifierParam<E: Engine> {
    /// The generator of G1.
    pub g: E::G1Affine,
    /// The generator of G2.
    pub h: E::G2Affine,
    /// \beta times the above generator of G2.
    pub beta_h: E::G2Affine,
}

/// Commitment Bi-KZG polynomial commitment scheme.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BiKZGCommitment<E: Engine> {
    /// the actual commitment is an affine point.
    pub com: E::G1Affine,
}

/// Proof for Bi-KZG polynomial commitment scheme.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BiKZGProof<E: Engine> {
    /// the actual proof is a pair of affine points.
    pub pi0: E::G1Affine,
    pub pi1: E::G1Affine,
}

impl<E: Engine> SerdeObject for BiKZGCommitment<E> {
    /// The purpose of unchecked functions is to read the internal memory representation
    /// of a type from bytes as quickly as possible. No sanitization checks are performed
    /// to ensure the bytes represent a valid object. As such this function should only be
    /// used internally as an extension of machine memory. It should not be used to deserialize
    /// externally provided data.
    fn from_raw_bytes_unchecked(bytes: &[u8]) -> Self {
        todo!("Implement this function")
    }
    fn from_raw_bytes(bytes: &[u8]) -> Option<Self> {
        todo!("Implement this function")
    }

    fn to_raw_bytes(&self) -> Vec<u8> {
        todo!("Implement this function")
    }

    /// The purpose of unchecked functions is to read the internal memory representation
    /// of a type from disk as quickly as possible. No sanitization checks are performed
    /// to ensure the bytes represent a valid object. This function should only be used
    /// internally when some machine state cannot be kept in memory (e.g., between runs)
    /// and needs to be reloaded as quickly as possible.
    fn read_raw_unchecked<R: Read>(reader: &mut R) -> Self {
        todo!("Implement this function")
    }
    fn read_raw<R: Read>(reader: &mut R) -> io::Result<Self> {
        todo!("Implement this function")
    }

    fn write_raw<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        todo!("Implement this function")
    }
}

impl<E: Engine> SerdeObject for BiKZGProof<E> {
    /// The purpose of unchecked functions is to read the internal memory representation
    /// of a type from bytes as quickly as possible. No sanitization checks are performed
    /// to ensure the bytes represent a valid object. As such this function should only be
    /// used internally as an extension of machine memory. It should not be used to deserialize
    /// externally provided data.
    fn from_raw_bytes_unchecked(bytes: &[u8]) -> Self {
        todo!("Implement this function")
    }
    fn from_raw_bytes(bytes: &[u8]) -> Option<Self> {
        todo!("Implement this function")
    }

    fn to_raw_bytes(&self) -> Vec<u8> {
        todo!("Implement this function")
    }

    /// The purpose of unchecked functions is to read the internal memory representation
    /// of a type from disk as quickly as possible. No sanitization checks are performed
    /// to ensure the bytes represent a valid object. This function should only be used
    /// internally when some machine state cannot be kept in memory (e.g., between runs)
    /// and needs to be reloaded as quickly as possible.
    fn read_raw_unchecked<R: Read>(reader: &mut R) -> Self {
        todo!("Implement this function")
    }
    fn read_raw<R: Read>(reader: &mut R) -> io::Result<Self> {
        todo!("Implement this function")
    }

    fn write_raw<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        todo!("Implement this function")
    }
}
