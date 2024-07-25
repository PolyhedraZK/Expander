use crate::{
    BinomialExtensionField, Field, FieldSerde, M31Ext3, SimdField, SimdM31, SimdM31Ext3, M31,
};

// pub trait GKRField: SimdField + BinomialExtensionField<3> + FieldSerde
// where
//     Self::BaseField: SimdField,
// {
//     type CircuitField: Field + FieldSerde;

//     type ChallengeField: BinomialExtensionField<3>;

//     // this is M31Ext3 * M31
//     fn simd_scalar_mul_ext_base_scalar(
//         a: &<Self as SimdField>::Scalar,
//         b: &<<Self as BinomialExtensionField<3>>::BaseField as SimdField>::Scalar,
//     ) -> <Self as SimdField>::Scalar;
// }

// impl GKRField for SimdM31Ext3 {
//     type CircuitField = M31;

//     type ChallengeField = M31Ext3;

//     // this is M31Ext3 * M31
//     #[inline(always)]
//     fn simd_scalar_mul_ext_base_scalar(
//         a: &<Self as SimdField>::Scalar, // M31Ext3
//         b: &<<Self as BinomialExtensionField<3>>::BaseField as SimdField>::Scalar, // M31
//     ) -> <Self as SimdField>::Scalar {
//         a.mul_by_base_field(b)
//     }
// }

// pub trait GKRConfig {
//     type CircuitField; //  e.g., M31
//     type ChallengeField; // e.g., M31Ext3
//     type Field; // e.g., SimdM31Ext3
// }
