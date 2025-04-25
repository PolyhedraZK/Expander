use goldilocks::{Goldilocks, GoldilocksExt2, GoldilocksExt2x8, Goldilocksx8};

use crate::{FieldEngine, FieldType};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GoldilocksExtConfig;

impl FieldEngine for GoldilocksExtConfig {
    const FIELD_TYPE: FieldType = FieldType::Goldilocks;

    const SENTINEL: [u8; 32] = [
        1, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];

    type CircuitField = Goldilocks;

    type SimdCircuitField = Goldilocksx8;

    type ChallengeField = GoldilocksExt2;

    type Field = GoldilocksExt2x8;
}
