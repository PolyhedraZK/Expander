use arith::{ExtensionField, Field};

use tiny_keccak::{Hasher, Keccak};

use super::FiatShamirFieldHash;

#[derive(Debug, Clone, Default)]
pub struct MIMCConstants<F: Field> {
    cts: Vec<F>,
    n_rounds: i64,
}

#[derive(Debug, Clone, Default)]
pub struct MIMCHasher<F: Field, ExtF: ExtensionField<BaseField = F>> {
    constants: MIMCConstants<ExtF>,
}

impl<F: Field, ExtF: ExtensionField<BaseField = F>> FiatShamirFieldHash<F, ExtF>
    for MIMCHasher<F, ExtF>
{
    fn new() -> Self {
        Self {
            constants: generate_mimc_constants::<ExtF>(),
        }
    }

    fn hash(&self, input: &[ExtF]) -> ExtF {
        let mut h = ExtF::ZERO;
        for a in input {
            let r = self.mimc5_hash(&h, a);
            h += r + a;
        }
        h
    }
}

impl<F: Field, ExtF: ExtensionField<BaseField = F>> MIMCHasher<F, ExtF> {
    #[inline(always)]
    pub fn pow5(x: ExtF) -> ExtF {
        let x2 = x * x;
        let x4 = x2 * x2;
        x4 * x
    }

    pub fn mimc5_hash(&self, h: &ExtF, x_in: &ExtF) -> ExtF {
        let mut x = *x_in;

        for i in 0..self.constants.n_rounds as usize {
            x = Self::pow5(x + h + self.constants.cts[i]);
        }
        x + h
    }
}

const SEED: &str = "seed";
pub fn generate_mimc_constants<F: Field>() -> MIMCConstants<F> {
    let mut keccak = Keccak::v256();
    let mut h = [0u8; 32];
    keccak.update(SEED.as_bytes());
    keccak.finalize(&mut h);
    let mut keccak = Keccak::v256();
    let mut h_iv = [0u8; 32];
    let seed_iv = format!("{}{}", SEED, "_iv");
    keccak.update(seed_iv.as_bytes());
    keccak.finalize(&mut h_iv);

    let n_rounds: i64 = 110;
    let cts = get_constants(SEED, n_rounds);
    MIMCConstants::<F> { cts, n_rounds }
}

pub fn get_constants<F: Field>(seed: &str, n_rounds: i64) -> Vec<F> {
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
