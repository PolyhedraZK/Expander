use crate::StructuredReferenceString;
use arith::FieldSerde;

#[derive(Clone, Debug, Default)]
pub struct PCSEmptyType {}

impl FieldSerde for PCSEmptyType {
    const SERIALIZED_SIZE: usize = 0;

    fn serialize_into<W: std::io::Write>(&self, _writer: W) -> arith::FieldSerdeResult<()> {
        Ok(())
    }

    fn deserialize_from<R: std::io::Read>(_reader: R) -> arith::FieldSerdeResult<Self> {
        Ok(Self {})
    }

    fn try_deserialize_from_ecc_format<R: std::io::Read>(
        _reader: R,
    ) -> arith::FieldSerdeResult<Self> {
        unimplemented!()
    }
}

impl StructuredReferenceString for PCSEmptyType {
    type PKey = PCSEmptyType;
    type VKey = PCSEmptyType;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (Self {}, Self {})
    }
}
