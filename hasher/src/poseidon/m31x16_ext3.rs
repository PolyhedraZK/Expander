use std::{
    io::{Read, Write},
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign},
};

use arith::{ExtensionField, Field, FieldForECC, FieldSerde, SimdField};
use mersenne31::{M31Ext3, M31x16, M31};

use crate::{FieldHasherState, PoseidonState};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PoseidonM31x16Ext3(M31x16);

impl FieldHasherState for PoseidonM31x16Ext3 {
    type InputF = M31;

    type OutputF = M31Ext3;

    const STATE_WIDTH: usize = 16;

    const STATE_NAME: &'static str = "Poseidon M31x16 Field Hasher State";

    fn from_elems(elems: &[Self::InputF]) -> Self {
        assert!(elems.len() <= Self::STATE_WIDTH);
        let mut local_copy = elems.to_vec();
        local_copy.resize(Self::STATE_WIDTH, Self::InputF::ZERO);
        Self(M31x16::pack(&local_copy))
    }

    fn to_elems(&self) -> Vec<Self::InputF> {
        self.0.unpack()
    }

    fn digest(&self) -> Self::OutputF {
        Self::OutputF::from_limbs(&self.0.unpack())
    }
}

impl FieldSerde for PoseidonM31x16Ext3 {
    const SERIALIZED_SIZE: usize = M31x16::SERIALIZED_SIZE;

    fn deserialize_from<R: Read>(reader: R) -> arith::FieldSerdeResult<Self> {
        let m31x16 = M31x16::deserialize_from(reader)?;
        Ok(Self(m31x16))
    }

    fn serialize_into<W: Write>(&self, writer: W) -> arith::FieldSerdeResult<()> {
        self.0.serialize_into(writer)
    }
}

impl Add for PoseidonM31x16Ext3 {
    type Output = PoseidonM31x16Ext3;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for PoseidonM31x16Ext3 {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl<'a> AddAssign<&'a Self> for PoseidonM31x16Ext3 {
    fn add_assign(&mut self, rhs: &'a Self) {
        self.0 += rhs.0
    }
}

impl Mul for PoseidonM31x16Ext3 {
    type Output = PoseidonM31x16Ext3;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl MulAssign for PoseidonM31x16Ext3 {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 *= rhs.0
    }
}

impl<'a> MulAssign<&'a Self> for PoseidonM31x16Ext3 {
    fn mul_assign(&mut self, rhs: &'a Self) {
        self.0 *= rhs.0
    }
}

impl PoseidonState<M31, M31Ext3> for PoseidonM31x16Ext3 {
    fn apply_mds_matrix(&mut self, mds_matrix: &[Self]) {
        let modulus = M31::MODULUS.as_u64();
        let mut res = [0u32; Self::STATE_WIDTH];

        res.iter_mut()
            .zip(mds_matrix.iter())
            .for_each(|(res, mds_col)| unsafe {
                let hadamard = *mds_col * *self;

                let u64_sum = transmute::<M31x16, [u32; Self::STATE_WIDTH]>(hadamard.0)
                    .iter()
                    .map(|&x| x as u64)
                    .sum::<u64>();
                *res = ((u64_sum & modulus) + (u64_sum >> 31)) as u32;
            });

        self.0 = unsafe { transmute::<[u32; Self::STATE_WIDTH], M31x16>(res) };
    }

    fn full_round_sbox(&mut self) {
        self.0 = self.0.exp(Self::SBOX_POW as u128);
    }

    fn partial_round_sbox(&mut self) {
        let mut buf = unsafe { transmute::<M31x16, [M31; Self::STATE_WIDTH]>(self.0) };
        buf[0] = buf[0].exp(Self::SBOX_POW as u128);
        self.0 = unsafe { transmute::<[M31; Self::STATE_WIDTH], M31x16>(buf) };
    }

    fn indexed_digest(&self, index: usize) -> M31Ext3 {
        M31Ext3::from_limbs(
            &self.to_elems()[index * Self::OUTPUT_ELEM_DEG..(index + 1) * Self::OUTPUT_ELEM_DEG],
        )
    }
}
