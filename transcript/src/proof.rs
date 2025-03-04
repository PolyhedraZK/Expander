use std::io::{Read, Write};

use serdes::{ArithSerde, SerdeResult};

/// Proof. In the serialized mode.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Proof {
    pub bytes: Vec<u8>,
}

impl Proof {
    #[inline(always)]
    pub fn append_u8_slice(&mut self, buffer: &[u8], size: usize) {
        self.bytes.extend_from_slice(&buffer[..size]);
    }
}

impl ArithSerde for Proof {
    const SERIALIZED_SIZE: usize = panic!("not implemented for Proof");

    #[inline(always)]
    fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
        (self.bytes.len() as u64).serialize_into(&mut writer)?;
        writer.write_all(&self.bytes)?;
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
        let proof_len = u64::deserialize_from(&mut reader)? as usize;
        let mut proof = vec![0u8; proof_len];
        reader.read_exact(&mut proof).unwrap();
        Ok(Self { bytes: proof })
    }
}
