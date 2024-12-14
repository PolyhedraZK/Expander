use std::mem::transmute;

use arith::{Field, FieldForECC, SimdField};
use mersenne31::{M31x16, M31};

use crate::PoseidonState;

const MATRIX_CIRC_MDS_16_SML_ROW: [u32; 16] =
    [1, 1, 51, 1, 11, 17, 2, 1, 101, 63, 15, 2, 67, 22, 13, 3];

impl PoseidonState<M31> for M31x16 {
    const STATE_WIDTH: usize = 16;

    fn apply_mds_matrix(&mut self, mds_matrix: &[Self]) {
        let modulus = M31::MODULUS.as_u64();
        let mut res = [0u32; Self::STATE_WIDTH];

        res.iter_mut()
            .zip(mds_matrix.iter())
            .for_each(|(res, mds_col)| unsafe {
                let hadamard = *mds_col * *self;

                let u64_sum = transmute::<M31x16, [u32; Self::STATE_WIDTH]>(hadamard)
                    .iter()
                    .map(|&x| x as u64)
                    .sum::<u64>();
                *res = ((u64_sum & modulus) + (u64_sum >> 31)) as u32;
            });

        *self = unsafe { transmute::<[u32; Self::STATE_WIDTH], M31x16>(res) };
    }

    fn full_round_sbox(&mut self) {
        *self = self.exp(Self::SBOX_EXP as u128);
    }

    fn partial_round_sbox(&mut self) {
        let mut buf = unsafe { transmute::<M31x16, [M31; Self::STATE_WIDTH]>(*self) };
        buf[0] = buf[0].exp(Self::SBOX_EXP as u128);
        *self = unsafe { transmute::<[M31; Self::STATE_WIDTH], M31x16>(buf) };
    }

    fn from_elems(elems: &[M31]) -> Self {
        M31x16::pack(elems)
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
