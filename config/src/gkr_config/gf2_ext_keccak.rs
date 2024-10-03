use arith::ExtensionField;
use gf2::{GF2x8, GF2};
use gf2_128::{GF2_128x8, GF2_128};
use transcript::Keccak256hasher;

use super::{FieldType, GKRConfig};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GF2ExtConfigKeccak;

impl GKRConfig for GF2ExtConfigKeccak {
    type CircuitField = GF2;

    type SimdCircuitField = GF2x8;

    type ChallengeField = GF2_128;

    type Field = GF2_128x8;

    type FiatShamirHashType = Keccak256hasher;

    const FIELD_TYPE: FieldType = FieldType::GF2;

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
        b_simd_ext.mul_by_base_field(a)
    }
}
