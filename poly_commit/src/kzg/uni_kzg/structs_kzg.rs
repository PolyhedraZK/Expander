use derivative::Derivative;
use gkr_engine::StructuredReferenceString;
use halo2curves::group::prime::PrimeCurveAffine;
use halo2curves::group::UncompressedEncoding;
use halo2curves::{pairing::Engine, CurveAffine};
use rayon::prelude::*;
use serdes::{ExpSerde, SerdeResult};
use std::io::{Read, Write};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct UniKZGCommitment<E: Engine>(pub E::G1Affine)
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>;

impl<E: Engine> AsRef<UniKZGCommitment<E>> for UniKZGCommitment<E>
where
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
{
    fn as_ref(&self) -> &UniKZGCommitment<E> {
        self
    }
}

// Derive macros does not work for associated types
impl<E: Engine> ExpSerde for UniKZGCommitment<E>
where
    E::G1Affine: ExpSerde + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
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
#[derive(Clone, Debug, PartialEq, Eq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct CoefFormUniKZGSRS<E: Engine>
where
    E::G1Affine: ExpSerde,
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2> + ExpSerde,
{
    /// power of \tau times the generators of G1, yielding
    /// \tau^i over G1 with i ranging in \[ 0, 2^n - 1 \]
    pub powers_of_tau: Vec<E::G1Affine>,
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

/// Custom ExpSerde implementation with parallel deserialization for fast SRS loading.
/// Uses UNCOMPRESSED format to avoid expensive square root computation during load.
impl<E: Engine> ExpSerde for CoefFormUniKZGSRS<E>
where
    E::G1Affine: ExpSerde
        + CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>
        + UncompressedEncoding
        + Send
        + Sync,
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2> + ExpSerde,
{
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        // Serialize length
        self.powers_of_tau.len().serialize_into(&mut writer)?;

        // Get uncompressed point size
        let point_size =
            std::mem::size_of::<<E::G1Affine as UncompressedEncoding>::Uncompressed>();
        let total_size = self.powers_of_tau.len() * point_size;

        // Pre-allocate buffer and convert all points to uncompressed format in parallel
        let mut buffer = vec![0u8; total_size];
        buffer
            .par_chunks_mut(point_size)
            .zip(self.powers_of_tau.par_iter())
            .for_each(|(chunk, point)| {
                let uncompressed = point.to_uncompressed();
                chunk.copy_from_slice(uncompressed.as_ref());
            });

        // Write all at once
        writer.write_all(&buffer)?;

        // Serialize tau_g2
        self.tau_g2.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        // Read length
        let len = usize::deserialize_from(&mut reader)?;

        // Uncompressed G1 point size (e.g. BN256: 64 bytes)
        let point_size =
            std::mem::size_of::<<E::G1Affine as UncompressedEncoding>::Uncompressed>();
        let total_bytes = len * point_size;

        let mut buffer = vec![0u8; total_bytes];
        reader.read_exact(&mut buffer)?;

        // Parse points in parallel - using from_uncompressed_unchecked (no square root needed)
        let powers_of_tau: Vec<E::G1Affine> = buffer
            .par_chunks(point_size)
            .map(|chunk| {
                let mut uncompressed =
                    <E::G1Affine as UncompressedEncoding>::Uncompressed::default();
                uncompressed.as_mut().copy_from_slice(chunk);
                E::G1Affine::from_uncompressed_unchecked(&uncompressed)
                    .into_option()
                    .expect("Invalid G1 point in SRS file")
            })
            .collect();

        // Read tau_g2
        let tau_g2 = E::G2Affine::deserialize_from(&mut reader)?;

        Ok(Self {
            powers_of_tau,
            tau_g2,
        })
    }
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
#[derive(Copy, Clone, Debug, PartialEq, Eq, ExpSerde)]
pub struct UniKZGVerifierParams<E: Engine>
where
    E::G2Affine: ExpSerde,
{
    /// \tau over G2
    pub tau_g2: E::G2Affine,
}

impl<E: Engine> From<&CoefFormUniKZGSRS<E>> for UniKZGVerifierParams<E>
where
    E::G1Affine: ExpSerde,
    E::G2Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G2> + ExpSerde,
{
    fn from(value: &CoefFormUniKZGSRS<E>) -> Self {
        Self {
            tau_g2: value.tau_g2,
        }
    }
}
