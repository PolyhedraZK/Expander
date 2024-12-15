use std::{
    mem::transmute,
    ops::{Add, AddAssign, Mul, MulAssign},
};

use arith::{ExtensionField, Field, FieldForECC, SimdField};
use mersenne31::{M31Ext3, M31x16, M31};

use crate::{FieldHasherState, PoseidonState};

const MATRIX_CIRC_MDS_16_SML_ROW: [u32; 16] =
    [1, 1, 51, 1, 11, 17, 2, 1, 101, 63, 15, 2, 67, 22, 13, 3];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PoseidonM31x16Ext3(M31x16);

impl FieldHasherState for PoseidonM31x16Ext3 {
    type InputF = M31;

    type Output = M31Ext3;

    const STATE_WIDTH: usize = 16;

    fn from_elems(elems: &[Self::InputF]) -> Self {
        Self(M31x16::pack(elems))
    }

    fn digest(&self) -> Self::Output {
        Self::Output::from_limbs(&self.0.unpack())
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
        self.0 = self.0.exp(Self::SBOX_EXP as u128);
    }

    fn partial_round_sbox(&mut self) {
        let mut buf = unsafe { transmute::<M31x16, [M31; Self::STATE_WIDTH]>(self.0) };
        buf[0] = buf[0].exp(Self::SBOX_EXP as u128);
        self.0 = unsafe { transmute::<[M31; Self::STATE_WIDTH], M31x16>(buf) };
    }

    fn mds_matrix() -> Vec<Self> {
        let doubled_buffer: Vec<_> = [
            MATRIX_CIRC_MDS_16_SML_ROW.as_slice(),
            MATRIX_CIRC_MDS_16_SML_ROW.as_slice(),
        ]
        .concat()
        .iter()
        .map(|t| From::from(*t))
        .collect();

        (0..Self::STATE_WIDTH)
            .map(|i| Self::from_elems(&doubled_buffer[i..i + Self::STATE_WIDTH]))
            .collect()
    }
}
