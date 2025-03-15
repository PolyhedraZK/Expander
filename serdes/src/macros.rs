#[macro_export]
macro_rules! exp_serde_for_number {
    ($int_type: ident,  $size_in_bytes: expr) => {
        impl ExpSerde for $int_type {
            const SERIALIZED_SIZE: usize = $size_in_bytes;

            /// serialize number into bytes
            fn serialize_into<W: Write>(&self, mut writer: W) -> SerdeResult<()> {
                writer.write_all(&self.to_le_bytes())?;
                Ok(())
            }

            /// deserialize bytes into number
            fn deserialize_from<R: Read>(mut reader: R) -> SerdeResult<Self> {
                let mut buffer = [0u8; Self::SERIALIZED_SIZE];
                reader.read_exact(&mut buffer)?;
                Ok($int_type::from_le_bytes(buffer))
            }
        }
    };
}
