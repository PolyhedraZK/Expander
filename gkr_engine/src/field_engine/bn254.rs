use crate::{FieldEngine, FieldType};
use arith::Fr;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BN254Config;

impl FieldEngine for BN254Config {
    const FIELD_TYPE: FieldType = FieldType::BN254;

    const SENTINEL: [u8; 32] = [
        1, 0, 0, 240, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88, 129, 129,
        182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48,
    ];

    type CircuitField = Fr;

    type ChallengeField = Fr;

    type Field = Fr;

    type SimdCircuitField = Fr;

    #[inline(always)]
    fn challenge_mul_circuit_field(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField {
        a * b
    }

    #[inline(always)]
    fn field_mul_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        a * b
    }

    #[inline(always)]
    fn field_add_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        *a + *b
    }

    #[inline(always)]
    fn field_add_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field {
        a + b
    }

    #[inline(always)]
    fn field_mul_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field {
        a * b
    }

    #[inline(always)]
    fn challenge_mul_field(a: &Self::ChallengeField, b: &Self::Field) -> Self::Field {
        a * b
    }

    #[inline(always)]
    fn circuit_field_into_field(a: &Self::CircuitField) -> Self::Field {
        *a
    }

    #[inline(always)]
    fn circuit_field_mul_simd_circuit_field(
        a: &Self::CircuitField,
        b: &Self::SimdCircuitField,
    ) -> Self::SimdCircuitField {
        *a * *b
    }

    #[inline(always)]
    fn circuit_field_to_simd_circuit_field(a: &Self::CircuitField) -> Self::SimdCircuitField {
        *a
    }
    #[inline(always)]
    fn simd_circuit_field_into_field(a: &Self::SimdCircuitField) -> Self::Field {
        *a
    }

    #[inline(always)]
    fn simd_circuit_field_mul_challenge_field(
        a: &Self::SimdCircuitField,
        b: &Self::ChallengeField,
    ) -> Self::Field {
        *a * b
    }
}
