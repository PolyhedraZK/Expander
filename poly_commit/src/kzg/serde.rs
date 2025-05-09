use halo2curves::{pairing::Engine, CurveAffine};
use serdes::{ExpSerde, SerdeResult};

use crate::*;

// Derive macros does not work for associated types
impl<E: Engine> ExpSerde for KZGCommitment<E>
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
