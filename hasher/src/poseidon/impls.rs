use std::fmt::Debug;

use arith::Field;
use tiny_keccak::{Hasher, Keccak};

use crate::{FiatShamirFieldHasher, PoseidonStateTrait};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoseidonPermutation<State: PoseidonStateTrait> {
    pub mds_matrix: Vec<State>,
    pub round_constants: Vec<State>,
}

pub const POSEIDON_SEED_PREFIX: &str = "poseidon_seed";

pub fn get_constants<State: PoseidonStateTrait>(round_num: usize) -> Vec<State> {
    let seed = format!(
        "{POSEIDON_SEED_PREFIX}_{}_{}",
        State::ElemT::NAME,
        State::STATE_WIDTH
    );

    let mut keccak = Keccak::v256();
    let mut buffer = [0u8; 32];
    keccak.update(seed.as_bytes());
    keccak.finalize(&mut buffer);

    (0..round_num)
        .map(|_| {
            let state_elems: Vec<_> = (0..State::STATE_WIDTH)
                .map(|_| {
                    let mut keccak = Keccak::v256();
                    keccak.update(&buffer);
                    keccak.finalize(&mut buffer);
                    State::ElemT::from_uniform_bytes(&buffer)
                })
                .collect();
            State::from_elems(&state_elems)
        })
        .collect()
}

pub const MATRIX_CIRC_MDS_8_SML_ROW: [u32; 8] = [7, 1, 3, 8, 8, 3, 4, 9];

pub const MATRIX_CIRC_MDS_12_SML_ROW: [u32; 12] = [1, 1, 2, 1, 8, 9, 10, 7, 5, 9, 4, 10];

pub const MATRIX_CIRC_MDS_16_SML_ROW: [u32; 16] =
    [1, 1, 51, 1, 11, 17, 2, 1, 101, 63, 15, 2, 67, 22, 13, 3];

pub fn get_mds_matrix<State: PoseidonStateTrait>() -> Vec<State> {
    let mds_first_row: &[u32] = match State::STATE_WIDTH {
        8 => &MATRIX_CIRC_MDS_8_SML_ROW,
        12 => &MATRIX_CIRC_MDS_12_SML_ROW,
        16 => &MATRIX_CIRC_MDS_16_SML_ROW,
        _ => unimplemented!("unsupported state width for MDS matrix"),
    };

    let buffer: Vec<_> = [mds_first_row, mds_first_row]
        .concat()
        .iter()
        .cloned()
        .map(From::from)
        .collect();

    (0..State::STATE_WIDTH)
        .map(|i| State::from_elems(&buffer[i..i + State::STATE_WIDTH]))
        .collect()
}
impl<State: PoseidonStateTrait> PoseidonPermutation<State> {
    fn new() -> Self {
        let total_rounds = State::FULL_ROUNDS + State::PARTIAL_ROUNDS;

        Self {
            mds_matrix: get_mds_matrix::<State>(),
            round_constants: get_constants::<State>(total_rounds),
        }
    }

    fn permute(&self, state: &mut State) {
        let half_full_rounds = State::FULL_ROUNDS / 2;
        let partial_ends = State::FULL_ROUNDS / 2 + State::PARTIAL_ROUNDS;

        (0..half_full_rounds).for_each(|i| {
            *state += &self.round_constants[i];
            state.apply_mds_matrix(&self.mds_matrix);
            state.full_round_sbox();
        });

        (half_full_rounds..partial_ends).for_each(|i| {
            *state += &self.round_constants[i];
            state.apply_mds_matrix(&self.mds_matrix);
            state.partial_round_sbox();
        });

        (partial_ends..half_full_rounds + partial_ends).for_each(|i| {
            *state += &self.round_constants[i];
            state.apply_mds_matrix(&self.mds_matrix);
            state.full_round_sbox();
        });
    }
}

impl<State: PoseidonStateTrait> FiatShamirFieldHasher<State::ElemT> for PoseidonPermutation<State> {
    const NAME: &'static str = "Poseidon Field Hasher";

    const STATE_CAPACITY: usize = State::CAPACITY;

    fn new() -> Self {
        Self::new()
    }

    fn hash_to_state(&self, input: &[State::ElemT]) -> Vec<State::ElemT> {
        let mut res = State::default();

        let mut elts = input.to_vec();
        elts.resize(elts.len().next_multiple_of(State::RATE), State::ElemT::ZERO);

        elts.chunks(State::RATE).for_each(|chunk| {
            let mut state_elts = vec![State::ElemT::ZERO; State::CAPACITY];
            state_elts.extend_from_slice(chunk);
            let state = State::from_elems(&state_elts);

            res += state;
            self.permute(&mut res);
        });

        res.to_elems()
    }
}

pub type PoseidonFiatShamirHasher<State> = PoseidonPermutation<State>;
