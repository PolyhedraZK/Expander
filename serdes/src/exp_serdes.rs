use std::io::{Read, Write};

use crate::SerdeResult;

/// Serde for Expander types such as proofs, witnesses and circuits.
pub trait ExpSerde: Sized {
    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, writer: W) -> SerdeResult<()>;

    /// deserialize bytes into field
    fn deserialize_from<R: Read>(reader: R) -> SerdeResult<Self>;
}
