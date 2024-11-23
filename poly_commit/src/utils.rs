use arith::FieldSerde;

use crate::StructuredReferenceString;

#[derive(Clone, Debug, Default, FieldSerde)]
pub struct PCSEmptyType {}

impl StructuredReferenceString for PCSEmptyType {
    type PKey = PCSEmptyType;
    type VKey = PCSEmptyType;

    fn into_keys(self) -> (Self::PKey, Self::VKey) {
        (Self {}, Self {})
    }
}
