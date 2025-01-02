use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Mul, MulAssign},
};

use arith::{Field, FieldForECC};
use tiny_keccak::{Hasher, Keccak};

use crate::FiatShamirFieldHasher;

pub trait PoseidonStateTrait:
    Sized
    + Default
    + Clone
    + Debug
    + Add<Self, Output = Self>
    + for<'a> Add<&'a Self, Output = Self>
    + AddAssign<Self>
    + for<'a> AddAssign<&'a Self>
    + Mul<Self, Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + MulAssign<Self>
    + for<'a> MulAssign<&'a Self>
{
    /// ElemT is the element (base field) type constructing the poseidon state instance.
    type ElemT: FieldForECC;

    /// SBOX_POW is a pow \alpha for poseidon sbox, that \alpha >= 3 and gcd(\alpha, p - 1) = 1,
    /// where p is the modulus of the prime field ElemT.
    const SBOX_POW: usize;

    /// FULL_ROUNDS in a poseidon permutation.
    const FULL_ROUNDS: usize;

    /// PARTIAL_ROUNDS in a poseidon permutation.
    const PARTIAL_ROUNDS: usize;

    /// STATE_WIDTH stands for the number of field elements that build up a poseidon state.
    const STATE_WIDTH: usize;

    /// CAPACITY is the number of output field elements (field here stands for InputF).
    ///
    /// We ensure (roughly) the collision resilience is 128 bits by FIELD_SIZE, i.e.,
    /// how many bits we need to store a field element.
    const CAPACITY: usize = 128 / <Self::ElemT as Field>::FIELD_SIZE * 2;

    /// RATE is the number of input elements in a round of sponge absorbing.
    /// The invariant here is RATE + CAPACITY = STATE_WIDTH
    const RATE: usize = Self::STATE_WIDTH - Self::CAPACITY;

    /// from_elems constructs an instance of field hasher state from the input field elements.
    fn from_elems(elems: &[Self::ElemT]) -> Self;

    /// to_elems is a conjugate method of from_elems, that separates the state into elems.
    fn to_elems(&self) -> Vec<Self::ElemT>;

    /// apply_mds_matrix applies MDS matrix back to the Poseidon state.
    fn apply_mds_matrix(&mut self, mds_matrix: &[Self]) {
        let res = mds_matrix
            .iter()
            .map(|mds_col| {
                let hadamard: Self = self.clone() * mds_col;
                hadamard.to_elems().iter().sum()
            })
            .collect::<Vec<_>>();

        *self = Self::from_elems(&res)
    }

    /// full_round_sbox applies x -> x^(sbox_exp) to all elements in the state.
    fn full_round_sbox(&mut self) {
        let mut elts = self.to_elems();
        elts.iter_mut()
            .for_each(|e| *e = e.exp(Self::SBOX_POW as u128));
        *self = Self::from_elems(&elts);
    }

    /// partial_round_sbox applies x -> x^(sbox_exp) to the first element in the state.
    fn partial_round_sbox(&mut self) {
        let mut elts = self.to_elems();
        elts[0] = elts[0].exp(Self::SBOX_POW as u128);
        *self = Self::from_elems(&elts);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoseidonPermutation<State: PoseidonStateTrait> {
    pub mds_matrix: Vec<State>,
    pub round_constants: Vec<State>,
}

const POSEIDON_SEED_PREFIX: &str = "poseidon_seed";

fn get_constants<State: PoseidonStateTrait>(round_num: usize) -> Vec<State> {
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

const MATRIX_CIRC_MDS_8_SML_ROW: [u32; 8] = [7, 1, 3, 8, 8, 3, 4, 9];

const MATRIX_CIRC_MDS_12_SML_ROW: [u32; 12] = [1, 1, 2, 1, 8, 9, 10, 7, 5, 9, 4, 10];

const MATRIX_CIRC_MDS_16_SML_ROW: [u32; 16] =
    [1, 1, 51, 1, 11, 17, 2, 1, 101, 63, 15, 2, 67, 22, 13, 3];

fn get_mds_matrix<State: PoseidonStateTrait>() -> Vec<State> {
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
