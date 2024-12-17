use std::marker::PhantomData;

use arith::{Field, FieldForECC};
use halo2curves::bn256::Fr;
use tiny_keccak::{Hasher, Keccak};

use crate::{FiatShamirSponge, FieldHasherState};

pub trait MiMCState<F: Field>:
    Field + FieldHasherState<InputF = F, OutputF = F> + From<F> + Into<F>
{
}

const MIMC_SEED: &str = "seed";

fn get_constants<F: Field, State: MiMCState<F>>(n_rounds: usize) -> Vec<State> {
    let mut keccak = Keccak::v256();
    let mut h = [0u8; 32];
    keccak.update(MIMC_SEED.as_bytes());
    keccak.finalize(&mut h);

    (0..n_rounds)
        .map(|_| {
            let mut keccak = Keccak::v256();
            keccak.update(&h);
            keccak.finalize(&mut h);

            // NOTE(ZF, HS): the behavior of gnark is taking the 256-bit hash
            // and store as big-endian mode, while our rust-runtime takes the same
            // 256-bit hash and store as little-endian mode.  The way to go for now
            // is to reverse the order to ensure the behavior matches on both ends.
            let mut h_reverse = h;
            h_reverse.reverse();

            State::from_uniform_bytes(&h_reverse)
        })
        .collect()
}

// NOTE(HS) we skip the FieldHasher implementation for MiMC, as essentially it is a block cipher.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MiMCSponge<F: Field, State: MiMCState<F>> {
    pub constants: Vec<State>,
    pub absorbed: State,

    _phantom: PhantomData<F>,
}

impl<F: Field, State: MiMCState<F>> MiMCSponge<F, State> {
    #[inline(always)]
    pub fn pow5(x: F) -> F {
        let x2 = x * x;
        let x4 = x2 * x2;
        x4 * x
    }

    pub fn mimc5_hash(&self, h: &F, x_in: &F) -> F {
        let mut x = *x_in;

        self.constants.iter().for_each(|ct| {
            x = Self::pow5(x + h + (*ct).into());
        });
        x + h
    }
}

impl<F: FieldForECC, State: MiMCState<F>> FiatShamirSponge<State> for MiMCSponge<F, State> {
    const NAME: &'static str = "MiMC Fiat-Shamir Sponge";

    fn new() -> Self {
        Self {
            constants: match State::STATE_NAME {
                Fr::STATE_NAME => get_constants::<F, State>(110),
                _ => unimplemented!("unsupported curve for MiMC"),
            },
            ..Default::default()
        }
    }

    fn update(&mut self, input: &[<State as FieldHasherState>::InputF]) {
        input.iter().for_each(|a| {
            let r = self.mimc5_hash(&self.absorbed.into(), a);
            self.absorbed = (self.absorbed.into() + r + a).into();
        })
    }

    fn is_squeezed(&self) -> bool {
        true
    }

    fn squeeze(&mut self) -> <State as FieldHasherState>::OutputF {
        self.absorbed.digest()
    }
}
