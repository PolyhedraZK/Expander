use arith::ExtensionField;
use babybear::{BabyBear, BabyBearExt3, BabyBearExt3x16, BabyBearx16};

use super::{FieldEngine, FieldType};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BabyBearExtConfig;

impl FieldEngine for BabyBearExtConfig {
    const FIELD_TYPE: FieldType = FieldType::BabyBear;

    const SENTINEL: [u8; 32] = [
        1, 0, 0, 120, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];

    type CircuitField = BabyBear;

    type SimdCircuitField = BabyBearx16;

    type ChallengeField = BabyBearExt3;

    type Field = BabyBearExt3x16;

    #[inline(always)]
    fn challenge_mul_circuit_field(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField {
        a.mul_by_base_field(b)
    }

    #[inline(always)]
    fn field_mul_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        *a * *b
    }

    #[inline(always)]
    fn field_add_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        *a + *b
    }

    #[inline(always)]
    fn field_add_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field {
        a.add_by_base_field(b)
    }

    #[inline(always)]
    fn field_mul_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field {
        a.mul_by_base_field(b)
    }

    #[inline(always)]
    fn challenge_mul_field(a: &Self::ChallengeField, b: &Self::Field) -> Self::Field {
        let a_simd = Self::Field::from(*a);
        a_simd * b
    }

    #[inline(always)]
    fn circuit_field_into_field(a: &Self::SimdCircuitField) -> Self::Field {
        Self::Field::from(*a)
    }

    #[inline(always)]
    fn circuit_field_mul_simd_circuit_field(
        a: &Self::CircuitField,
        b: &Self::SimdCircuitField,
    ) -> Self::SimdCircuitField {
        Self::SimdCircuitField::from(*a) * *b
    }

    #[inline(always)]
    fn circuit_field_to_simd_circuit_field(a: &Self::CircuitField) -> Self::SimdCircuitField {
        Self::SimdCircuitField::from(*a)
    }

    #[inline(always)]
    fn simd_circuit_field_into_field(a: &Self::SimdCircuitField) -> Self::Field {
        Self::Field::from(*a)
    }

    #[inline(always)]
    fn simd_circuit_field_mul_challenge_field(
        a: &Self::SimdCircuitField,
        b: &Self::ChallengeField,
    ) -> Self::Field {
        let b_simd_ext = Self::Field::from(*b);
        Self::Field {
            v: [
                b_simd_ext.v[0] * a,
                b_simd_ext.v[1] * a,
                b_simd_ext.v[2] * a,
            ],
        }
    }
}
