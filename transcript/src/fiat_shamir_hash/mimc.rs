use arith::{FiatShamirFieldHasher, Field};

use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, Clone, Default)]
pub struct MIMCHasher<F: Field> {
    constants: Vec<F>,
}

impl<F: Field> FiatShamirFieldHasher<F> for MIMCHasher<F> {
    fn new() -> Self {
        let constants = generate_mimc_constants::<F>();
        Self { constants }
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

impl<F: Field> MIMCHasher<F> {
    #[inline(always)]
    pub fn pow5(x: F) -> F {
        let x2 = x * x;
        let x4 = x2 * x2;
        x4 * x
    }

    #[inline(always)]
    pub fn mimc5_hash(&self, h: &F, x_in: &F) -> F {
        let mut x = *x_in;

        self.constants.iter().for_each(|c| {
            x = Self::pow5(x + h + c);
        });
        x + h
    }
}

const SEED: &str = "seed";

pub fn generate_mimc_constants<F: Field>() -> Vec<F> {
    let n_rounds: usize = 110;
    get_constants(SEED, n_rounds)
}

pub fn get_constants<F: Field>(seed: &str, n_rounds: usize) -> Vec<F> {
    let mut keccak = Keccak::v256();
    let mut h = [0u8; 32];
    keccak.update(seed.as_bytes());
    keccak.finalize(&mut h);

    (0..n_rounds)
        .map(|_| {
            let mut keccak = Keccak::v256();
            keccak.update(&h);
            keccak.finalize(&mut h);

            // big endian -> little endian, in order to match the one in gnark
            // or probably we can change the implementation there
            let mut h_reverse = h;
            h_reverse.reverse();

            F::from_uniform_bytes(&h_reverse)
        })
        .collect()
}
