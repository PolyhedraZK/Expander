// use std::ops::{Add, Mul};

use crate::Field;

// pub trait Pack<Scalar: Field + Send, PackScalar: SimdField<Scalar>> {
//     // TODO: test correct in BN254 Fr
//     fn pack(base_vec: &[PackScalar]) -> Self {
//         assert_eq!(base_vec.len() * PackScalar::PACK_SIZE, Self::PACK_SIZE);
//         let temp: Vec<_> = base_vec.to_vec();
//         unsafe { *(temp.as_ptr() as *const Self) }
//     }
// }

/// Configurations for the SimdField.
pub trait SimdField<Scalar: Field + Send>: From<Scalar> + Field {
    /// Field for the challenge. Can be self.
    // type Scalar: Field + Send;

    /// Pack size (width) for the SIMD instruction
    const PACK_SIZE: usize;

    /// scale self with the challenge
    fn scale(&self, challenge: &Scalar) -> Self;

    /// pack a vec of simd field with same scalar field into self
    fn pack_from_simd<PackF>(simd_vec: &[PackF]) -> Self
    where
        PackF: SimdField<Scalar>,
    {
        assert_eq!(simd_vec.len() * PackF::PACK_SIZE, Self::PACK_SIZE);
        let mut temp: Vec<_> = simd_vec.to_vec();
        // NOTE(HS) this method `pack_from_simd` was introduced with
        // a motivation of packing multiple GF2x8 into a GF2x64 or a GF2x128.
        // The reverse here is to ensure that packing all these GF2x8s into a
        // final GF2x64 can unpack into a vec of GF2s, that is the same order
        // as a concatenation of unpacked GF2s from GF2x8.
        // temp.reverse();

        unsafe { *(temp.as_ptr() as *const Self) }
    }

    /// pack a vec of scalar field into self
    fn pack(base_vec: &[Scalar]) -> Self;

    /// unpack into a vector.
    fn unpack(&self) -> Vec<Scalar>;

    /// horizontally sum all packed scalars
    fn horizontal_sum(&self) -> Scalar {
        self.unpack().iter().sum()
    }

    fn index(&self, pos: usize) -> Scalar {
        self.unpack()[pos]
    }
}

/*
pub trait PackMul<Scalar: Field + Send, PackedField: SimdField<Scalar>, ResField: Field>:
    Mul<Scalar, Output = ResField>
{
    fn pack_mul(&self, rhs: PackedField, res: &mut [ResField]) {
        assert_eq!(PackedField::PACK_SIZE, res.len());
        let up = rhs.unpack();
        for (i, u) in up.iter().enumerate() {
            res[i] = *self * *u;
        }
    }
}

pub trait PackMulAssign<Scalar: Field + Send, PackedField: SimdField<Scalar>>:
    Mul<Scalar, Output = Self>
{
    fn pack_mul_assign(&mut self, rhs: PackedField);
}

pub trait PackAdd<Scalar: Field + Send, PackedField: SimdField<Scalar>, ResField: Field> :
    Add<Scalar, Output = ResField>
{
    fn pack_add(&self, rhs: PackedField, res: &mut [ResField]);
}

pub trait PackAddAssign<SelfScalar: Field + Send, Scalar: Field + Send, PackedField: SimdField<Scalar>>:
    Add<Scalar, Output = Self> + SimdField<SelfScalar>
{
    fn pack_add_assign(&self, rhs: PackedField) {
        assert_eq!(Self::PACK_SIZE, rhs.len());
        let up = rhs.unpack();
        for (i, u) in up.iter().enumerate() {
            self[i] = self[i] + u;
        }
    }
}
 */