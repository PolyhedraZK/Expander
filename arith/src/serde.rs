use std::io::{Read, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FieldSerdeError {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Deserialization failure")]
    DeserializeError,
}

pub type FieldSerdeResult<T> = std::result::Result<T, FieldSerdeError>;

/// Serde for Fields
pub trait FieldSerde: Sized {
    const SERIALIZED_SIZE: usize;

    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, writer: W) -> FieldSerdeResult<()>;

    /// deserialize bytes into field
    fn deserialize_from<R: Read>(reader: R) -> FieldSerdeResult<Self>;

    /// deserialize bytes into field following ecc format
    fn try_deserialize_from_ecc_format<R: Read>(reader: R) -> FieldSerdeResult<Self>;
}

macro_rules! field_serde_for_integer {
    ($int_type: ident, $size_in_bytes: expr) => {
        impl FieldSerde for $int_type {
            /// size of the serialized bytes
            const SERIALIZED_SIZE: usize = $size_in_bytes;

            /// serialize u64 into bytes
            fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
                writer.write_all(&self.to_le_bytes())?;
                Ok(())
            }

            /// deserialize bytes into u64
            fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
                let mut buffer = [0u8; Self::SERIALIZED_SIZE];
                reader.read_exact(&mut buffer)?;
                Ok($int_type::from_le_bytes(buffer))
            }

            fn try_deserialize_from_ecc_format<R: Read>(_reader: R) -> FieldSerdeResult<Self> {
                unimplemented!("not implemented")
            }
        }
    };
}

field_serde_for_integer!(u64, 8);
field_serde_for_integer!(usize, 8);
field_serde_for_integer!(u8, 1);

// Consider use const generics after it gets stable
impl FieldSerde for [u64; 4] {
    const SERIALIZED_SIZE: usize = 32;

    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        for i in self {
            writer.write_all(&i.to_le_bytes())?;
        }
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut ret = [0u64; 4];
        let mut buffer = [0u8; u64::SERIALIZED_SIZE];

        for r in &mut ret {
            reader.read_exact(&mut buffer)?;
            *r = u64::from_le_bytes(buffer);
        }
        Ok(ret)
    }

    fn try_deserialize_from_ecc_format<R: Read>(_reader: R) -> FieldSerdeResult<Self> {
        unimplemented!()
    }
}
