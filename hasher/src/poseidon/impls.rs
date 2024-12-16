use std::{
    marker::PhantomData,
    ops::{Add, AddAssign, Mul, MulAssign},
};

use arith::{Field, FieldForECC};
use mersenne31::M31;
use tiny_keccak::{Hasher, Keccak};

use crate::{FieldHasher, FieldHasherSponge, FieldHasherState};

use super::{compile_time_alpha, PoseidonM31x16Ext3};

pub trait PoseidonState<F: FieldForECC, OF: Field>:
    FieldHasherState<InputF = F, OutputF = OF>
    + Add<Self, Output = Self>
    + AddAssign<Self>
    + for<'a> AddAssign<&'a Self>
    + Mul<Self, Output = Self>
    + MulAssign<Self>
    + for<'a> MulAssign<&'a Self>
{
    const SBOX_EXP: usize = compile_time_alpha::<F>();

    const CAPACITY: usize = 128 / F::FIELD_SIZE * 2;

    const RATE: usize = Self::STATE_WIDTH - Self::CAPACITY;

    fn apply_mds_matrix(&mut self, mds_matrix: &[Self]);

    fn full_round_sbox(&mut self);

    fn partial_round_sbox(&mut self);

    // NOTE(HS) this is not quite a good place for MDS matrix generation here
    // as it should be something like passing in state width and field, we generate
    // a viable MDS instance...   but I am just tired of writing stuffs at this point,
    // and the method appears here just like me existing in this place without proper reason.
    // forgive me for me being nihilstic being myself here.
    // btw this method is only called in parameter generation, and will not be invoked in the
    // real deal of proving/hashing process, so yall chill.
    fn mds_matrix() -> Vec<Self>;
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

impl<F: FieldForECC, OF: Field, State: PoseidonState<F, OF>> PoseidonParams<F, OF, State> {
    pub(crate) fn parameterized_new(half_full_rounds: usize, partial_rounds: usize) -> Self {
        let total_rounds = 2 * half_full_rounds + partial_rounds;

        Self {
            half_full_rounds,
            partial_rounds,

            mds_matrix: State::mds_matrix(),
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
pub struct PoseidonHasherSponge<InputF, OutputF, State>
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

impl<F: FieldForECC, OF: Field, State: PoseidonState<F, OF>>
    FieldHasherSponge<State, PoseidonParams<F, OF, State>> for PoseidonHasherSponge<F, OF, State>
{
    const NAME: &'static str = "Poseidon Field Hasher Sponge";

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

        for chunk in absorb_tbd.chunks(State::RATE) {
            if chunk.len() < State::RATE {
                self.absorbing = chunk.to_vec()
            } else {
                let mut unpacked_msg =
                    vec![<State as FieldHasherState>::InputF::ZERO; State::STATE_WIDTH];
                unpacked_msg[State::CAPACITY..].copy_from_slice(chunk);
                let new_state = State::from_elems(&unpacked_msg);

                self.absorbed += new_state;
                self.params.permute(&mut self.absorbed);
            }
        }
    }

    fn squeeze(&mut self) -> <State as FieldHasherState>::OutputF {
        if self.absorbing.is_empty() {
            let mut tailing_elems =
                vec![<State as FieldHasherState>::InputF::ZERO; State::STATE_WIDTH];
            tailing_elems[State::CAPACITY..State::CAPACITY + self.absorbing.len()]
                .copy_from_slice(&self.absorbing);
            let new_state = State::from_elems(&tailing_elems);

            self.absorbed += new_state;
            self.params.permute(&mut self.absorbed);
            self.output_index = 0;
        }

        let next_output_index = self.output_index + State::OUTPUT_ELEM_DEG;
        if next_output_index <= State::CAPACITY {
            let absorbed_unpacked = self.absorbed.to_elems();
            let mut phony_unpacked = absorbed_unpacked[self.output_index..].to_vec();
            phony_unpacked.resize(
                State::STATE_WIDTH,
                <State as FieldHasherState>::InputF::ZERO,
            );
            let phony_absorbed = State::from_elems(&phony_unpacked);
            self.output_index = next_output_index;
            phony_absorbed.digest()
        } else {
            self.output_index = 0;
            self.params.permute(&mut self.absorbed);
            self.squeeze()
        }
    }
}
