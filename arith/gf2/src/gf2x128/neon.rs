#[derive(Clone, Copy, Debug)]
pub struct NeonGF2x128 {
    pub(crate) v: uint32x4_t,
}

impl FieldSerde for NeonGF2_128 {
    const SERIALIZED_SIZE: usize = 16;

    #[inline(always)]
    fn serialize_into<W: std::io::Write>(&self, mut writer: W) -> FieldSerdeResult<()> {
        unsafe { writer.write_all(transmute::<uint32x4_t, [u8; 16]>(self.v).as_ref())? };
        Ok(())
    }

    #[inline(always)]
    fn deserialize_from<R: std::io::Read>(mut reader: R) -> FieldSerdeResult<Self> {
        let mut u = [0u8; 16];
        reader.read_exact(&mut u)?;
        unsafe {
            Ok(NeonGF2_128 {
                v: transmute::<[u8; 16], uint32x4_t>(u),
            })
        }
    }

    #[inline]
    fn try_deserialize_from_ecc_format<R: std::io::Read>(mut reader: R) -> FieldSerdeResult<Self>
    where
        Self: Sized,
    {
        let mut u = [0u8; 32];
        reader.read_exact(&mut u)?;
        Ok(unsafe {
            NeonGF2_128 {
                v: transmute::<[u8; 16], uint32x4_t>(u[..16].try_into().unwrap()),
            }
        })
    }
}
// TODO: FieldSerde

// TODO: Field

// TODO: SimdField
