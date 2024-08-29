use arith::parallelize;
use halo2curves::{
    ff::{PrimeField, WithSmallOrderMulGroup},
    fft::best_fft,
};

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

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CosetFFTDomain<F: PrimeField> {
    pub(crate) omega: F,
    pub(crate) omega_inv: F,
    pub(crate) zeta: F,
    pub(crate) zeta_inv: F,
    pub(crate) n: usize,
    pub(crate) log_n: usize,
    pub(crate) ext_n: usize,
    pub(crate) log_ext_n: usize,
    pub(crate) one_over_ext_n: F,
}

impl<F: PrimeField + WithSmallOrderMulGroup<3>> CosetFFTDomain<F> {
    #[inline]
    pub fn new(log_n: usize) -> Self {
        let n = 1 << log_n;
        let ext_n = n << 2;
        let r = F::ROOT_OF_UNITY;
        let omega = r.pow_vartime(&[1u64 << (F::S - 2 - log_n as u32) as u64]);
        let omega_inv = omega.invert().unwrap(); // safe unwrap
        let one_over_ext_n = F::from(ext_n as u64).invert().unwrap(); // safe unwrap
        let zeta = F::ZETA;
        let zeta_inv = zeta.square();

        Self {
            omega,
            omega_inv,
            zeta,
            zeta_inv,
            n: n as usize,
            log_n,
            ext_n: ext_n as usize,
            log_ext_n: log_n + 2,
            one_over_ext_n,
        }
    }

    #[inline]
    pub fn coset_fft(&self, a: &[F]) -> Vec<F> {
        assert_eq!(a.len(), self.n);
        let mut a = a.to_vec();
        self.distribute_powers_zeta(&mut a, true);
        a.resize(self.ext_n, F::ZERO);
        best_fft(a.as_mut_slice(), self.omega, self.log_ext_n as u32);
        a
    }

    #[inline]
    pub fn coset_ifft(&self, a: &[F]) -> Vec<F> {
        assert_eq!(a.len(), self.ext_n);
        let mut a = a.to_vec();
        best_fft(a.as_mut_slice(), self.omega_inv, self.log_ext_n as u32);
        a.iter_mut().for_each(|a| *a *= &self.one_over_ext_n);
        self.distribute_powers_zeta(&mut a, false);
        a
    }

    /// Given a slice of group elements `[a_0, a_1, a_2, ...]`, this returns
    /// `[a_0, [zeta]a_1, [zeta^2]a_2, a_3, [zeta]a_4, [zeta^2]a_5, a_6, ...]`,
    /// where zeta is a cube root of unity in the multiplicative subgroup with
    /// order (p - 1), i.e. zeta^3 = 1.
    ///
    /// `into_coset` should be set to `true` when moving into the coset,
    /// and `false` when moving out. This toggles the choice of `zeta`.
    pub fn distribute_powers_zeta(&self, a: &mut [F], into_coset: bool) {
        let coset_powers = if into_coset {
            [self.zeta, self.zeta_inv]
        } else {
            [self.zeta_inv, self.zeta]
        };
        parallelize(a, |a, mut index| {
            for a in a {
                // Distribute powers to move into/from coset
                let i = index % (coset_powers.len() + 1);
                if i != 0 {
                    *a *= &coset_powers[i - 1];
                }
                index += 1;
            }
        });
    }
}
