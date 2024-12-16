use arith::{Field, FieldForECC};
use tiny_keccak::{Hasher, Keccak};

use crate::{FieldHasher, FieldHasherState};

// TODO ... MIMC implementation

#[derive(Debug, Clone, Default)]
pub struct MIMCConstants<F: Field> {
    pub cts: Vec<F>,
    pub n_rounds: i64,
}

const SEED: &str = "seed";
fn generate_mimc_constants<F: Field>() -> MIMCConstants<F> {
    let n_rounds: i64 = 110;
    let cts = get_constants(SEED, n_rounds);
    MIMCConstants::<F> { cts, n_rounds }
}

fn get_constants<F: Field>(seed: &str, n_rounds: i64) -> Vec<F> {
    let mut cts: Vec<F> = Vec::new();

    let mut keccak = Keccak::v256();
    let mut h = [0u8; 32];
    keccak.update(seed.as_bytes());
    keccak.finalize(&mut h);

    for _ in 0..n_rounds {
        let mut keccak = Keccak::v256();
        keccak.update(&h);
        keccak.finalize(&mut h);

        // big endian -> little endian, in order to match the one in gnark
        // or probably we can change the implementation there
        let mut h_reverse = h;
        h_reverse.reverse();

        cts.push(F::from_uniform_bytes(&h_reverse));
    }
    cts
}

#[derive(Debug, Clone, Default)]
pub struct MIMCHasher<F: Field> {
    pub constants: MIMCConstants<F>,
}

impl<F: Field> MIMCHasher<F> {
    #[inline(always)]
    pub fn pow5(x: F) -> F {
        let x2 = x * x;
        let x4 = x2 * x2;
        x4 * x
    }

    pub fn mimc5_hash(&self, h: &F, x_in: &F) -> F {
        let mut x = *x_in;

        for i in 0..self.constants.n_rounds as usize {
            x = Self::pow5(x + h + self.constants.cts[i]);
        }
        x + h
    }
}

impl<F: FieldForECC, HashState: FieldHasherState<InputF = F, OutputF = F>> FieldHasher<HashState>
    for MIMCHasher<F>
{
    const NAME: &'static str = "MiMC Field Hasher";

    fn new() -> Self {
        Self {
            constants: generate_mimc_constants::<F>(),
        }
    }

    fn permute(&self, _state: &mut HashState) {
        todo!()
    }

    fn hash(&self, input: &[F]) -> F {
        let mut h = F::ZERO;
        for a in input {
            let r = self.mimc5_hash(&h, a);
            h += r + a;
        }
        h
    }
}
