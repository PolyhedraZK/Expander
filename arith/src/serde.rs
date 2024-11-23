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
    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, writer: W) -> FieldSerdeResult<()>;

    /// deserialize bytes into field
    fn deserialize_from<R: Read>(reader: R) -> FieldSerdeResult<Self>;
}

macro_rules! field_serde_for_number {
    ($int_type: ident, $size_in_bytes: expr) => {
        impl FieldSerde for $int_type {
            /// serialize u64 into bytes
            fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
                writer.write_all(&self.to_le_bytes())?;
                Ok(())
            }

            /// deserialize bytes into u64
            fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
                let mut buffer = [0u8; $size_in_bytes];
                reader.read_exact(&mut buffer)?;
                Ok($int_type::from_le_bytes(buffer))
            }
        }
    };
}

field_serde_for_number!(u64, 8);
field_serde_for_number!(usize, 8);
field_serde_for_number!(u8, 1);
field_serde_for_number!(f64, 8);

impl<V: FieldSerde> FieldSerde for Vec<V> {
    fn serialize_into<W: Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        self.len().serialize_into(&mut writer)?;
        for v in self.iter() {
            v.serialize_into(&mut writer)?;
        }
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut v = Self::default();
        let len = usize::deserialize_from(&mut reader)?;
        for _ in 0..len {
            v.push(V::deserialize_from(&mut reader)?);
        }
        Ok(v)
    }
}

macro_rules! field_serde_for_num_array {
    ($num_type: ident, $num_size: expr, $array_len: expr) => {
        impl FieldSerde for [$num_type; $array_len] {
            fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
                for i in self {
                    writer.write_all(&i.to_le_bytes())?;
                }
                Ok(())
            }

            fn deserialize_from<R: std::io::Read>(mut reader: R) -> FieldSerdeResult<Self> {
                let mut ret: [$num_type; $array_len] = [0; $array_len];
                let mut buffer = [0u8; $num_size];

                for r in &mut ret {
                    reader.read_exact(&mut buffer)?;
                    *r = $num_type::from_le_bytes(buffer);
                }
                Ok(ret)
            }
        }
    };
}

field_serde_for_num_array!(u64, 8, 4);
field_serde_for_num_array!(u8, 1, 64);
field_serde_for_num_array!(u8, 1, 32);
