use std::{fmt::Debug, ops::{Add, Mul}};

use arith::{ExtensionField, Field, SimdField};
use transcript::Transcript;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    M31,
    BN254,
    GF2,
}

pub trait GKRFieldConfig: Default + Debug + Clone + Send + Sync + 'static 
{

    /// Enum type for Self::Field
    const FIELD_TYPE: FieldType;

    // TODO: used for self parallel
    // type CircuitUnitField: Field + Send;

    /// Field type for the circuit, e.g., M31
    type CircuitField: Field + Send;

    /// Simd field for circuit, e.g., M31x16
    type SimdCircuitField: SimdField<Self::CircuitField> 
        + Send
        + Mul<Self::CircuitField, Output = Self::SimdCircuitField>;
        // + Mul<Self::ChallengeField, Output = Self::Field>;

    /// Base field for the challenge
    type BaseField: Field + Send
        + Mul<Self::CircuitField, Output = Self::BaseField>;
    type SimdBaseField: SimdField<Self::BaseField> + Send;

    /// Field type for the challenge, e.g., M31Ext3
    type ChallengeField: ExtensionField<BaseField = Self::BaseField> 
        + Send 
        + Mul<Self::CircuitField, Output = Self::ChallengeField>
        + Mul<Self::SimdCircuitField, Output = Self::Field>
        + Mul<Self::Field, Output = Self::Field>
        + Mul<Self::BaseField, Output = Self::ChallengeField>
        + Mul<Self::SimdBaseField, Output = Self::Field>;
        // TODO: used for self parallel
        // + PackMul<Self::CircuitUnitField, Self::CircuitField, Self::ChallengeField>;

    /// Main field type for the scheme, e.g., M31Ext3x16
    type Field: ExtensionField<BaseField = Self::SimdBaseField>
        + SimdField<Self::ChallengeField>
        + Send
        + From<Self::SimdCircuitField>
        + Mul<Self::CircuitField, Output = Self::Field> 
        + Add<Self::CircuitField, Output = Self::Field> 
        + Mul<Self::SimdCircuitField, Output = Self::Field> 
        + Add<Self::SimdCircuitField, Output = Self::Field> 
        + Mul<Self::ChallengeField, Output = Self::Field>
        + Mul<Self::BaseField, Output = Self::Field>
        + Mul<Self::SimdBaseField, Output = Self::Field>
        + Add<Self::SimdBaseField, Output = Self::Field>;


    /*
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
    ) -> Self::Field; */

    /// The pack size for the simd circuit field, e.g., 16 for M31x16
    fn get_field_pack_size() -> usize {
        Self::SimdCircuitField::PACK_SIZE
    }

    // TODO: maybe error in BN254
    /*
    fn generate_circuit_field_element<F: ExtensionField, T: Transcript<F>>(transcript: &mut T) -> Self::CircuitField {
        let bytes_sampled = transcript.generate_challenge_u8_slice(32);
        let mut buffer = [0u8; 32];
        buffer[..32].copy_from_slice(&bytes_sampled);

        Self::CircuitField::from_uniform_bytes(&buffer)
    }

    fn generate_circuit_field_elements<F: ExtensionField, T: Transcript<F>>(transcript: &mut T, num_elems: usize) -> Vec<Self::CircuitField> {
        let mut circuit_fs = Vec::with_capacity(num_elems);
        (0..num_elems).for_each(|_| circuit_fs.push(Self::generate_circuit_field_element(transcript)));
        circuit_fs
    } */

    // TODO: used for self parallel
    // if the circuit field is binary
    // fn is_binary_circuit() -> bool {
    //     false
    // }

    // TODO: used for self parallel
    // the number of the witnesses in a circuit field
    // fn circuit_pack_size() -> usize {
    //     1
    // }
}