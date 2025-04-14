use mersenne31::{M31Ext3, M31Ext3x16, M31x16, M31};

use crate::{FieldEngine, FieldType};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct M31ExtConfig;

impl FieldEngine for M31ExtConfig {
    const FIELD_TYPE: FieldType = FieldType::M31;

    const SENTINEL: [u8; 32] = [
        255, 255, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];

    type CircuitField = M31;

    type SimdCircuitField = M31x16;

    type ChallengeField = M31Ext3;

    type Field = M31Ext3x16;

}
