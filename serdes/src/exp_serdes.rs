use std::{
    collections::HashMap,
    hash::Hash,
    io::{Read, Write},
};

use ethnum::U256;

use crate::{ArithSerde, SerdeResult};

/// Serde for Expander types such as proofs, witnesses and circuits.
pub trait ExpSerde: Sized {
    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, writer: W) -> SerdeResult<()>;

    /// deserialize bytes into field
    fn deserialize_from<R: Read>(reader: R) -> SerdeResult<Self>;
}

impl<V: ExpSerde> ExpSerde for Vec<V> {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        <usize as ArithSerde>::serialize_into(&self.len(), &mut writer)?;
        for v in self.iter() {
            v.serialize_into(&mut writer)?;
        }
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut v = Self::default();
        let len = usize::deserialize_from(&mut reader)?;
        for _ in 0..len {
            v.push(V::deserialize_from(&mut reader)?);
        }
        Ok(v)
    }
}

impl ExpSerde for () {
    fn serialize_into<W: std::io::Write>(&self, _writer: W) -> SerdeResult<()> {
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(_reader: R) -> SerdeResult<Self> {
        Ok(())
    }
}

impl ExpSerde for U256 {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        writer.write_all(&self.to_le_bytes())?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut bytes = [0u8; 32];
        reader.read_exact(&mut bytes)?;
        Ok(Self::from_le_bytes(bytes))
    }
}

impl<K: ExpSerde + Eq + Hash, V: ExpSerde> ExpSerde for HashMap<K, V> {
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.len().serialize_into(&mut writer)?;
        for (k, v) in self.iter() {
            k.serialize_into(&mut writer)?;
            v.serialize_into(&mut writer)?;
        }
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let len = usize::deserialize_from(&mut reader)?;
        let mut map = HashMap::with_capacity(len);
        for _ in 0..len {
            let k = K::deserialize_from(&mut reader)?;
            let v = V::deserialize_from(&mut reader)?;
            map.insert(k, v);
        }
        Ok(map)
    }
}
