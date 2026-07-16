#[macro_export]
macro_rules! exp_serde_for_number {
    ($int_type: ident,  $size_in_bytes: expr) => {
        impl ExpSerde for $int_type {
            /// serialize number into bytes
            fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
                writer.write_all(&self.to_le_bytes())?;
                Ok(())
            }

            /// deserialize bytes into number
            fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
                let mut buffer = [0u8; $size_in_bytes];
                reader.read_exact(&mut buffer)?;
                Ok($int_type::from_le_bytes(buffer))
            }
        }
    };
}

#[macro_export]
macro_rules! exp_serde_for_generic_slices {
    ($size: expr) => {
        impl<S: ExpSerde> ExpSerde for [S; $size] {
            fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
                // Fast path for u8 arrays: bulk write
                if std::mem::size_of::<S>() == 1 && std::mem::align_of::<S>() == 1 {
                    let bytes = unsafe {
                        std::slice::from_raw_parts(self.as_ptr() as *const u8, $size)
                    };
                    writer.write_all(bytes)?;
                    return Ok(());
                }
                for s in self.iter() {
                    s.serialize_into(&mut writer)?;
                }
                Ok(())
            }

            fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
                // Fast path for u8 arrays: bulk read
                if std::mem::size_of::<S>() == 1 && std::mem::align_of::<S>() == 1 {
                    let mut arr = [0u8; $size];
                    reader.read_exact(&mut arr)?;
                    return Ok(unsafe { std::mem::transmute_copy(&arr) });
                }
                let mut ret = Vec::with_capacity($size);
                for _ in 0..$size {
                    ret.push(S::deserialize_from(&mut reader)?);
                }
                ret.try_into().map_err(|_| SerdeError::DeserializeError)
            }
        }
    };
}
