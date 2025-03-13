use std::fmt::Debug;

use arith::{ExtensionField, Field, FieldForECC, SimdField};

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    M31,
    BN254,
    GF2,
}

pub trait GKRFieldConfig: Default + Debug + Clone + Send + Sync + 'static {
    /// Enum type for Self::Field
    const FIELD_TYPE: FieldType;

    /// Field type for the circuit, e.g., M31
    type CircuitField: Field + FieldForECC + Send;

    /// Field type for the challenge, e.g., M31Ext3
    type ChallengeField: ExtensionField<BaseField = Self::CircuitField> + Send;

    /// Main field type for the scheme, e.g., M31Ext3x16
    type Field: ExtensionField<BaseField = Self::SimdCircuitField>
        + SimdField<Scalar = Self::ChallengeField>
        + Send;

    /// Simd field for circuit, e.g., M31x16
    type SimdCircuitField: SimdField<Scalar = Self::CircuitField> + Send;

    /// API to allow for multiplications between the challenge and the circuit field
    fn challenge_mul_circuit_field(
        a: &Self::ChallengeField,
        b: &Self::CircuitField,
    ) -> Self::ChallengeField;

    /// API to allow for multiplications between the main field and the circuit field
    fn field_mul_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field;

    /// API to allow for addition between the main field and the circuit field
    fn field_add_circuit_field(a: &Self::Field, b: &Self::CircuitField) -> Self::Field;

    /// API to allow multiplications between the main field and the simd circuit field
    fn field_add_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field;

    /// API to allow multiplications between the main field and the simd circuit field
    fn field_mul_simd_circuit_field(a: &Self::Field, b: &Self::SimdCircuitField) -> Self::Field;

    /// API to allow for multiplications between the challenge and the main field
    fn challenge_mul_field(a: &Self::ChallengeField, b: &Self::Field) -> Self::Field;

    /// API to allow for multiplications between the challenge and the simd circuit field
    fn circuit_field_into_field(a: &Self::SimdCircuitField) -> Self::Field;

    /// API to allow for multiplications between the simd circuit field and the challenge
    fn circuit_field_mul_simd_circuit_field(
        a: &Self::CircuitField,
        b: &Self::SimdCircuitField,
    ) -> Self::SimdCircuitField;

    /// Convert a circuit field to a simd circuit field
    fn circuit_field_to_simd_circuit_field(a: &Self::CircuitField) -> Self::SimdCircuitField;

    /// Convert a simd circuit field to a circuit field
    fn simd_circuit_field_into_field(a: &Self::SimdCircuitField) -> Self::Field;

    /// API to allow for multiplications between the simd circuit field and the challenge
    fn simd_circuit_field_mul_challenge_field(
        a: &Self::SimdCircuitField,
        b: &Self::ChallengeField,
    ) -> Self::Field;

    /// The pack size for the simd circuit field, e.g., 16 for M31x16
    fn get_field_pack_size() -> usize {
        Self::SimdCircuitField::PACK_SIZE
    }
}
