use arith::{Field, SimdField};
use gf2::{GF2x16, GF2};
use mersenne31::{M31Ext3, M31Ext3x16, M31x16, M31};

use crate::{FieldType, GKRFieldConfig};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct M31ExtBinaryConfig;

impl GKRFieldConfig for M31ExtBinaryConfig {
    const FIELD_TYPE: FieldType = FieldType::M31;

    // type CircuitUnitField = GF2;
    type CircuitField = GF2;

    type SimdCircuitField = GF2x16;

    type BaseField = M31;
    type SimdBaseField = M31x16;

    type ChallengeField = M31Ext3;

    type Field = M31Ext3x16;

}
