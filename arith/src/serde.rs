use std::io::{Read, Write};

/// Serde for Fields
pub trait FieldSerde: Sized {
    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, writer: W) -> std::result::Result<(), std::io::Error>;

    /// size of the serialized bytes
    fn serialized_size() -> usize;

    /// deserialize bytes into field
    fn deserialize_from<R: Read>(reader: R) -> std::result::Result<Self, std::io::Error>;

    /// deserialize bytes into field following ecc format
    fn try_deserialize_from_ecc_format<R: Read>(
        reader: R,
    ) -> std::result::Result<Self, std::io::Error>;
}

impl FieldSerde for u64 {
    /// serialize u64 into bytes
    fn serialize_into<W: Write>(&self, mut writer: W) -> std::result::Result<(), std::io::Error> {
        writer.write_all(&self.to_le_bytes())
    }

    /// size of the serialized bytes
    fn serialized_size() -> usize {
        8
    }

    /// deserialize bytes into u64
    fn deserialize_from<R: Read>(mut reader: R) -> std::result::Result<Self, std::io::Error> {
        let mut buffer = [0u8; 8];
        reader.read_exact(&mut buffer)?;
        Ok(u64::from_le_bytes(buffer))
    }

    fn try_deserialize_from_ecc_format<R: Read>(
        _reader: R,
    ) -> std::result::Result<Self, std::io::Error> {
        unimplemented!("not implemented for u64")
    }
}
