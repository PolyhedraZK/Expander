use std::io::{Read, Write};

use ethnum::U256;
use halo2curves::{
    bn256::{Fr, G1Affine, G2Affine},
    group::GroupEncoding,
};

use crate::{exp_serde_for_number, SerdeError, SerdeResult};

/// Serde for Arithmetic types such as field and group operations
pub trait ExpSerde: Sized {
    const SERIALIZED_SIZE: usize;

    /// serialize self into bytes
    fn serialize_into<W: Write>(&self, writer: W) -> SerdeResult<()>;

    /// deserialize bytes into field
    fn deserialize_from<R: Read>(reader: R) -> SerdeResult<Self>;
}

impl ExpSerde for () {
    const SERIALIZED_SIZE: usize = 0;

    fn serialize_into<W: std::io::Write>(&self, _writer: W) -> SerdeResult<()> {
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(_reader: R) -> SerdeResult<Self> {
        Ok(())
    }
}

exp_serde_for_number!(u64, 8);
exp_serde_for_number!(usize, 8);
exp_serde_for_number!(u8, 1);
exp_serde_for_number!(f64, 8);
exp_serde_for_number!(u128, 16);
exp_serde_for_number!(U256, 32);

impl<V: ExpSerde> ExpSerde for Vec<V> {
    const SERIALIZED_SIZE: usize = unimplemented!();

    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.len().serialize_into(&mut writer)?;
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

// Consider use const generics after it gets stable
impl ExpSerde for [u64; 4] {
    const SERIALIZED_SIZE: usize = 32;

    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        for i in self {
            writer.write_all(&i.to_le_bytes())?;
        }
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut ret = [0u64; 4];
        let mut buffer = [0u8; u64::SERIALIZED_SIZE];

        for r in &mut ret {
            reader.read_exact(&mut buffer)?;
            *r = u64::from_le_bytes(buffer);
        }
        Ok(ret)
    }
}

impl ExpSerde for Fr {
    const SERIALIZED_SIZE: usize = 32;

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        writer.write_all(self.to_bytes().as_ref())?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let mut buffer = [0u8; Self::SERIALIZED_SIZE];
        reader.read_exact(&mut buffer)?;
        match Fr::from_bytes(&buffer).into_option() {
            Some(v) => Ok(v),
            None => Err(SerdeError::DeserializeError),
        }
    }
}

impl ExpSerde for G1Affine {
    const SERIALIZED_SIZE: usize = 32;

    fn serialize_into<W: Write>(&self, writer: W) -> SerdeResult<()> {
        let bytes = self.to_bytes().as_ref().to_vec();
        bytes.serialize_into(writer)
    }

    fn deserialize_from<R: Read>(reader: R) -> SerdeResult<Self> {
        let bytes: Vec<u8> = Vec::deserialize_from(reader)?;
        if bytes.len() != Self::SERIALIZED_SIZE {
            return Err(SerdeError::DeserializeError);
        }

        let mut encoding = <Self as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(bytes.as_ref());
        match G1Affine::from_bytes(&encoding).into_option() {
            Some(a) => Ok(a),
            None => Err(SerdeError::DeserializeError),
        }
    }
}

impl ExpSerde for G2Affine {
    const SERIALIZED_SIZE: usize = 64;

    fn serialize_into<W: Write>(&self, writer: W) -> SerdeResult<()> {
        let bytes = self.to_bytes().as_ref().to_vec();
        bytes.serialize_into(writer)
    }

    fn deserialize_from<R: Read>(reader: R) -> SerdeResult<Self> {
        let bytes: Vec<u8> = Vec::deserialize_from(reader)?;
        if bytes.len() != Self::SERIALIZED_SIZE {
            return Err(SerdeError::DeserializeError);
        }

        let mut encoding = <Self as GroupEncoding>::Repr::default();
        encoding.as_mut().copy_from_slice(bytes.as_ref());
        match G2Affine::from_bytes(&encoding).into_option() {
            Some(a) => Ok(a),
            None => Err(SerdeError::DeserializeError),
        }
    }
}

impl<T1: ExpSerde, T2: ExpSerde> ExpSerde for (T1, T2) {
    const SERIALIZED_SIZE: usize = T1::SERIALIZED_SIZE + T2::SERIALIZED_SIZE;

    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        self.0.serialize_into(&mut writer)?;
        self.1.serialize_into(&mut writer)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let t1 = T1::deserialize_from(&mut reader)?;
        let t2 = T2::deserialize_from(&mut reader)?;
        Ok((t1, t2))
    }
}
