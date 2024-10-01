use transcript::Transcript;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasefoldParam<T> {
    pub rate_bits: usize,
    pub verifier_queries: usize,
    pub _marker: std::marker::PhantomData<T>,
}

impl<T: Transcript> BasefoldParam<T> {
    pub fn new(rate_bits: usize) -> Self {
        // TODO: this number is arbitrary, need further analysis.
        let verifier_queries = 80;

        Self {
            rate_bits,
            verifier_queries,
            _marker: std::marker::PhantomData,
        }
    }

    #[inline]
    /// Generate a list of positions that we want to open the polynomial at.
    pub fn iopp_challenges(&self, num_vars: usize, transcript: &mut T) -> Vec<usize> {
        let iopp_challenge_bitmask = (1 << self.codeword_bits(num_vars)) - 1;

        // NOTE: Fiat-Shamir sampling an IOPP query point ranging [0, 2^codeword_bits - 1].
        transcript
            .generate_challenge_index_vector(self.verifier_queries)
            .map(|c| c & iopp_challenge_bitmask)
            .collect()
    }

    #[inline]
    pub fn codeword_bits(&self, num_vars: usize) -> usize {
        self.rate_bits + num_vars
    }
}
