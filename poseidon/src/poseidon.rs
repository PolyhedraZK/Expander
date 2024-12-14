use std::{
    marker::PhantomData,
    ops::{Add, AddAssign, Mul, MulAssign},
};

use arith::{Field, FieldForECC};
use mersenne31::{M31x16, M31};
use tiny_keccak::{Hasher, Keccak};

const fn compile_time_gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

const fn compile_time_alpha<F: FieldForECC>() -> usize {
    let modulus = F::MODULUS.as_usize();

    let mut alpha: usize = 5;
    while compile_time_gcd(alpha, modulus) != 1 {
        alpha += 2
    }
    alpha
}

pub trait PoseidonState<F: FieldForECC>:
    Sized
    + Clone
    + Copy
    + Default
    + Add<Self, Output = Self>
    + AddAssign<Self>
    + Mul<Self, Output = Self>
    + MulAssign<Self>
{
    const SBOX_EXP: usize = compile_time_alpha::<F>();

    const STATE_WIDTH: usize;

    fn apply_mds_matrix(&mut self, mds_matrix: &[Self]);

    fn full_round_sbox(&mut self);

    fn partial_round_sbox(&mut self);

    fn from_elems(elems: &[F]) -> Self;

    fn mds_matrix() -> Vec<Self>;

    // TODO(HS) extract field/extension-field API
}

#[derive(Debug, Clone, Default)]
pub struct PoseidonParams<BaseF, State>
where
    BaseF: FieldForECC,
    State: PoseidonState<BaseF>,
{
    pub half_full_rounds: usize,
    pub partial_rounds: usize,

    pub mds_matrix: Vec<State>,
    pub round_constants: Vec<State>,

    _phantom_base_field: PhantomData<BaseF>,
}

const POSEIDON_SEED: &str = "poseidon_seed";

pub fn get_constants<F: FieldForECC, State: PoseidonState<F>>(round_num: usize) -> Vec<State> {
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

impl<F: FieldForECC, State: PoseidonState<F>> PoseidonParams<F, State> {
    pub(crate) fn full_parameterized_new(half_full_rounds: usize, partial_rounds: usize) -> Self {
        let total_rounds = 2 * half_full_rounds + partial_rounds;

        Self {
            half_full_rounds,
            partial_rounds,

            mds_matrix: State::mds_matrix(),
            round_constants: get_constants::<F, State>(total_rounds),

            _phantom_base_field: PhantomData,
        }
    }

    pub fn new() -> Self {
        match (F::NAME, State::STATE_WIDTH) {
            (M31::NAME, M31x16::STATE_WIDTH) => Self::full_parameterized_new(4, 22),
            _ => unimplemented!("unimplemented as types for Poseidon instantiation unsupported"),
        }
    }

    pub fn permute(&self, state: &mut State) {
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
