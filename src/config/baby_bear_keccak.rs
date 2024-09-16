use super::{FieldType, GKRConfig};
use crate::Keccak256hasher;
use arith::{BabyBear, BabyBearExt4, ExtensionField};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BabyBearConfigKeccak;

// TODO: Replace Field, SimdCircuitField with SIMD field types
impl GKRConfig for BabyBearConfigKeccak {
    type CircuitField = BabyBear;

    type ChallengeField = BabyBearExt4;

    type Field = BabyBearExt4;

    type SimdCircuitField = BabyBear;

    type FiatShamirHashType = Keccak256hasher;

    const FIELD_TYPE: FieldType = FieldType::BabyBear;

    fn challenge_mul_circuit_field(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField {
        a.mul_by_base_field(b)
    }

    fn field_mul_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        a.mul_by_base_field(b)
    }

    fn field_add_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field {
        a.add_by_base_field(b)
    }

    fn field_add_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field {
        a.add_by_base_field(b)
    }

    fn field_mul_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field {
        a.mul_by_base_field(b)
    }

    fn challenge_mul_field(a: &Self::ChallengeField, b: &Self::Field) -> Self::Field {
        a * b
    }

    fn circuit_field_into_field(a: &Self::SimdCircuitField) -> Self::Field {
        Self::Field::from(*a)
    }

    fn circuit_field_mul_simd_circuit_field(
        a: &Self::CircuitField,
        b: &Self::SimdCircuitField,
    ) -> Self::SimdCircuitField {
        a * b
    }

    fn circuit_field_to_simd_circuit_field(a: &Self::CircuitField) -> Self::SimdCircuitField {
        *a
    }

    fn simd_circuit_field_into_field(a: &Self::SimdCircuitField) -> Self::Field {
        Self::Field::from(*a)
    }

    fn simd_circuit_field_mul_challenge_field(
        a: &Self::SimdCircuitField,
        b: &Self::ChallengeField,
    ) -> Self::Field {
        b.mul_by_base_field(a)
    }
}
