use halo2curves::{ff::PrimeField, fft::best_fft};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FFTDomain<F: PrimeField> {
    pub(crate) omega: F,
    pub(crate) omega_inv: F,
    pub(crate) one_over_n: F,
    pub(crate) n: usize,
    pub(crate) log_n: usize,
}

impl<F: PrimeField> FFTDomain<F> {
    #[inline]
    pub fn new(log_n: usize) -> Self {
        let n = 1 << log_n;
        let r = F::ROOT_OF_UNITY;
        let omega = r.pow_vartime(&[1u64 << (F::S - log_n as u32) as u64]);
        let omega_inv = omega.invert().unwrap(); // safe unwrap
        let one_over_n = F::from(n as u64).invert().unwrap(); // safe unwrap
        Self {
            omega,
            omega_inv,
            one_over_n,
            n: n as usize,
            log_n,
        }
    }

    #[inline]
    pub fn fft_in_place(&self, a: &mut [F]) {
        assert!(a.len() == self.n);
        best_fft(a, self.omega, self.log_n as u32);
    }

    #[inline]
    pub fn fft(&self, a: &[F]) -> Vec<F> {
        let mut a = a.to_vec();
        self.fft_in_place(&mut a);
        a
    }

    #[inline]
    pub fn ifft_in_place(&self, a: &mut [F]) {
        assert!(a.len() == self.n);
        best_fft(a, self.omega_inv, self.log_n as u32);
        a.iter_mut().for_each(|a| *a *= &self.one_over_n);
    }

    #[inline]
    pub fn ifft(&self, a: &[F]) -> Vec<F> {
        let mut a = a.to_vec();
        self.ifft_in_place(&mut a);
        a
    }
}
