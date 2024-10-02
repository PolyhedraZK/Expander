use arith::{FFTField, Field};
use transcript::{FiatShamirHash, Transcript};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasefoldParam<T, H> {
    pub rate_bits: usize,
    pub verifier_queries: usize,
    pub transcript: std::marker::PhantomData<T>,
    pub hasher: std::marker::PhantomData<H>,
}

impl<T, H> BasefoldParam<T, H>
where
    T: Transcript<H>,
    H: FiatShamirHash,
{
    pub fn new(rate_bits: usize) -> Self {
        // TODO: this number is arbitrary, need further analysis.
        let verifier_queries = 80;

        Self {
            rate_bits,
            verifier_queries,
            transcript: std::marker::PhantomData,
            hasher: std::marker::PhantomData,
        }
    }

    #[inline]
    /// Generate a list of positions that we want to open the polynomial at.
    pub fn iopp_challenges(&self, num_vars: usize, transcript: &mut T) -> Vec<usize> {
        let iopp_challenge_bitmask = (1 << self.codeword_bits(num_vars)) - 1;

        // NOTE: Fiat-Shamir sampling an IOPP query point ranging [0, 2^codeword_bits - 1].
        transcript
            .generate_challenge_index_vector(self.verifier_queries)
            .iter()
            .map(|c| c & iopp_challenge_bitmask)
            .collect()
    }

    #[inline]
    pub fn codeword_bits(&self, num_vars: usize) -> usize {
        self.rate_bits + num_vars
    }

    #[inline]
    pub fn t_term<F: FFTField>(&self, num_vars: usize, round: usize, index: usize) -> F {
        let t = F::two_adic_generator(self.codeword_bits(num_vars));
        let round_gen = F::two_adic_generator(self.codeword_bits(num_vars) - round);
        round_gen.exp(index as u128)
    }
}
