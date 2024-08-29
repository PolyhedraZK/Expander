use arith::parallelize;
use halo2curves::{
    ff::{BatchInvert, PrimeField, WithSmallOrderMulGroup},
    fft::best_fft,
};

pub const LOG_EXT_DEGREE: usize = 1;

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
        let omega = r.pow_vartime([1u64 << (F::S - log_n as u32) as u64]);
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
    /// roots of extension field
    pub(crate) omega: F,
    pub(crate) omega_inv: F,

    // zeta^3 == 1
    pub(crate) zeta: F,
    pub(crate) zeta_inv: F,

    // dimension of base domain
    pub(crate) n: usize,
    pub(crate) log_n: usize,

    // dimension of extended domain
    pub(crate) ext_n: usize,
    pub(crate) log_ext_n: usize,

    // 1/ext_n
    pub(crate) one_over_ext_n: F,

    // evaluation of x^n-1 at the coset domain, inverted
    pub(crate) t_eval_inv: Vec<F>,
}

impl<F: PrimeField + WithSmallOrderMulGroup<3>> CosetFFTDomain<F> {
    #[inline]
    pub fn new(log_n: usize) -> Self {
        let n = 1 << log_n;
        let log_ext_n = log_n + LOG_EXT_DEGREE;
        let ext_n = n << LOG_EXT_DEGREE;
        let r = F::ROOT_OF_UNITY;
        let omega = r.pow_vartime([1u64 << (F::S - log_ext_n as u32) as u64]);
        let omega_inv = omega.invert().unwrap(); // safe unwrap
        let one_over_ext_n = F::from(ext_n).invert().unwrap(); // safe unwrap
        let zeta = F::ZETA;
        let zeta_inv = zeta.square();

        let mut t_eval_inv = Vec::with_capacity(1 << (log_ext_n - log_n));
        {
            // Compute the evaluations of t(X) = X^n - 1 in the coset evaluation domain.
            // We don't have to compute all of them, because it will repeat.
            let orig = F::ZETA.pow_vartime([n]);
            let step = omega.pow_vartime([n]);
            let mut cur = orig;
            loop {
                t_eval_inv.push(cur);
                cur *= &step;
                if cur == orig {
                    break;
                }
            }
            assert_eq!(t_eval_inv.len(), 1 << (log_ext_n - log_n));

            // Subtract 1 from each to give us t_evaluations[i] = t(zeta * extended_omega^i)
            for coeff in &mut t_eval_inv {
                *coeff -= &F::ONE;
            }

            // Invert, because we're dividing by this polynomial.
            // We invert in a batch, below.
        }
        t_eval_inv.batch_invert();

        Self {
            omega,
            omega_inv,
            zeta,
            zeta_inv,
            n: n as usize,
            log_n,
            ext_n: ext_n as usize,
            log_ext_n,
            one_over_ext_n,
            t_eval_inv,
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

    /// This divides the polynomial (in the extended domain) by the vanishing
    /// polynomial of the $2^k$ size domain.
    pub fn divide_by_vanishing_poly(&self, a: &mut [F]) {
        assert_eq!(a.len(), self.ext_n);

        // Divide to obtain the quotient polynomial in the coset evaluation
        // domain.
        parallelize(a, |h, mut index| {
            for h in h {
                *h *= &self.t_eval_inv[index % self.t_eval_inv.len()];
                index += 1;
            }
        });
    }
}
