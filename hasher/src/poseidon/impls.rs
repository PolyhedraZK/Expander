use std::{
    marker::PhantomData,
    ops::{Add, AddAssign, Mul, MulAssign},
};

use arith::{Field, FieldForECC};
use mersenne31::M31;
use tiny_keccak::{Hasher, Keccak};

use crate::{FiatShamirSponge, FieldHasher, FieldHasherState, PoseidonM31x16Ext3};

use super::compile_time::compile_time_alpha;

pub trait PoseidonState<F: FieldForECC, OF: Field>:
    FieldHasherState<InputF = F, OutputF = OF>
    + Add<Self, Output = Self>
    + AddAssign<Self>
    + for<'a> AddAssign<&'a Self>
    + Mul<Self, Output = Self>
    + MulAssign<Self>
    + for<'a> MulAssign<&'a Self>
{
    /// SBOX_POW is the lowest exponential pow for poseidon sbox.
    const SBOX_POW: usize = compile_time_alpha::<F>();

    /// CAPACITY is the nubmer of output field elements (field here stands for InputF).
    ///
    /// We ensure (roughly) the collision resilience is 128 bits by FIELD_SIZE, i.e.,
    /// how many bits we need to store a field element.
    const CAPACITY: usize = 128 / F::FIELD_SIZE * 2;

    /// RATE is the number of input elements in a round of sponge absorbing.
    /// The invariant here is RATE + CAPACITY = STATE_WIDTH
    const RATE: usize = Self::STATE_WIDTH - Self::CAPACITY;

    /// apply_mds_matrix applies MDS matrix back to the Poseidon state.
    fn apply_mds_matrix(&mut self, mds_matrix: &[Self]);

    /// full_round_sbox applies x -> x^(sbox_exp) to all elements in the state.
    fn full_round_sbox(&mut self);

    /// partial_round_sbox applies x -> x^(sbox_exp) to the first element in the state.
    fn partial_round_sbox(&mut self);

    /// index_digest index into the state and take a range of elements from the state,
    /// fit into the output state by the ratio of output field size / input field size.
    fn indexed_digest(&self, index: usize) -> OF;
}

#[derive(Debug, Clone, Default)]
pub struct PoseidonParams<InputF, OutputF, State>
where
    InputF: FieldForECC,
    OutputF: Field,
    State: PoseidonState<InputF, OutputF>,
{
    pub half_full_rounds: usize,
    pub partial_rounds: usize,

    pub mds_matrix: Vec<State>,
    pub round_constants: Vec<State>,

    _phantom_base_field: PhantomData<InputF>,
    _phantom_output_field: PhantomData<OutputF>,
}

const POSEIDON_SEED: &str = "poseidon_seed";

fn get_constants<F: FieldForECC, OF: Field, State: PoseidonState<F, OF>>(
    round_num: usize,
) -> Vec<State> {
    let seed = format!("{POSEIDON_SEED}_{}_{}", F::NAME, State::STATE_WIDTH);

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
                    F::from_uniform_bytes(&buffer)
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

fn get_mds_matrix<F: FieldForECC, OF: Field, State: PoseidonState<F, OF>>() -> Vec<State> {
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

impl<F: FieldForECC, OF: Field, State: PoseidonState<F, OF>> PoseidonParams<F, OF, State> {
    pub(crate) fn parameterized_new(half_full_rounds: usize, partial_rounds: usize) -> Self {
        let total_rounds = 2 * half_full_rounds + partial_rounds;

        Self {
            half_full_rounds,
            partial_rounds,

            mds_matrix: get_mds_matrix::<F, OF, State>(),
            round_constants: get_constants::<F, OF, State>(total_rounds),

            _phantom_base_field: PhantomData,
            _phantom_output_field: PhantomData,
        }
    }
}

impl<F: FieldForECC, OF: Field, State: PoseidonState<F, OF>> FieldHasher<State>
    for PoseidonParams<F, OF, State>
{
    const NAME: &'static str = "Poseidon Field Hasher";

    fn new() -> Self {
        match (F::NAME, State::STATE_WIDTH) {
            (M31::NAME, PoseidonM31x16Ext3::STATE_WIDTH) => Self::parameterized_new(4, 22),
            _ => unimplemented!("unimplemented as types for Poseidon instantiation unsupported"),
        }
    }

    fn permute(&self, state: &mut State) {
        let partial_ends = self.half_full_rounds + self.partial_rounds;

        (0..self.half_full_rounds).for_each(|i| {
            *state += self.round_constants[i];
            state.apply_mds_matrix(&self.mds_matrix);
            state.full_round_sbox();
        });

        (self.half_full_rounds..partial_ends).for_each(|i| {
            *state += self.round_constants[i];
            state.apply_mds_matrix(&self.mds_matrix);
            state.partial_round_sbox();
        });

        (partial_ends..self.half_full_rounds + partial_ends).for_each(|i| {
            *state += self.round_constants[i];
            state.apply_mds_matrix(&self.mds_matrix);
            state.full_round_sbox();
        });
    }
}

#[derive(Debug, Clone, Default)]
pub struct PoseidonSponge<InputF, OutputF, State>
where
    InputF: FieldForECC,
    OutputF: Field,
    State: PoseidonState<InputF, OutputF>,
{
    pub params: PoseidonParams<InputF, OutputF, State>,

    pub absorbed: State,
    pub absorbing: Vec<InputF>,

    pub output_index: usize,

    _phantom: PhantomData<OutputF>,
}

impl<F: FieldForECC, OF: Field, State: PoseidonState<F, OF>> FiatShamirSponge<State>
    for PoseidonSponge<F, OF, State>
{
    const NAME: &'static str = "Poseidon Fiat-Shamir Sponge";

    fn new() -> Self {
        Self {
            params: PoseidonParams::new(),
            ..Default::default()
        }
    }

    fn update(&mut self, inputs: &[<State as FieldHasherState>::InputF]) {
        // NOTE: reset the output index on taking new inputs
        self.output_index = 0;

        let mut absorb_tbd = self.absorbing.clone();
        absorb_tbd.extend_from_slice(inputs);
        self.absorbing.clear();

        absorb_tbd.chunks(State::RATE).for_each(|chunk| {
            if chunk.len() < State::RATE {
                self.absorbing = chunk.to_vec()
            } else {
                let mut unpacked_msg = State::default().to_elems();
                unpacked_msg[State::CAPACITY..].copy_from_slice(chunk);
                let new_state = State::from_elems(&unpacked_msg);

                self.absorbed += new_state;
                self.params.permute(&mut self.absorbed);
            }
        })
    }

    fn squeeze(&mut self) -> <State as FieldHasherState>::OutputF {
        if !self.absorbing.is_empty() {
            let mut tailing_elems =
                vec![<State as FieldHasherState>::InputF::ZERO; State::STATE_WIDTH];
            tailing_elems[State::CAPACITY..State::CAPACITY + self.absorbing.len()]
                .copy_from_slice(&self.absorbing);
            let new_state = State::from_elems(&tailing_elems);

            self.absorbed += new_state;
            self.params.permute(&mut self.absorbed);
            self.output_index = 0;
            self.absorbing.clear();
        }

        let next_output_starts = self.output_index + State::OUTPUT_ELEM_DEG;
        if next_output_starts <= State::CAPACITY {
            let digest_index = self.output_index / State::OUTPUT_ELEM_DEG;
            let res = self.absorbed.indexed_digest(digest_index);
            self.output_index = next_output_starts;
            return res;
        }
        self.output_index = 0;
        self.params.permute(&mut self.absorbed);
        self.squeeze()
    }
}
