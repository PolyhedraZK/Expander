use arith::Field;

#[derive(Debug, Clone, PartialEq)]
pub struct EqPolynomial<F> {
    _phantom: std::marker::PhantomData<F>,
}

impl<F: Field> EqPolynomial<F> {
    #[inline(always)]
    fn eq(x: &F, y: &F) -> F {
        // x * y + (1 - x) * (1 - y)
        let xy = *x * y;
        xy + xy - x - y + F::one()
    }

    // #[inline(always)]
    // fn eq_3(x: &F, y: &F, z: &F) -> F {
    //     // TODO: extend this
    //     *x * y * z + (F::one() - x) * (F::one() - y) * (F::one() - z)
    // }

    #[inline(always)]
    pub(crate) fn eq_vec(xs: &[F], ys: &[F]) -> F {
        xs.iter()
            .zip(ys.iter())
            .map(|(x, y)| Self::eq(x, y))
            .product()
    }

    // #[inline(always)]
    // pub(crate) fn eq_vec_3(xs: &[F], ys: &[F], zs: &[F]) -> F {
    //     xs.iter()
    //         .zip(ys.iter())
    //         .zip(zs.iter())
    //         .map(|((x, y), z)| Self::eq_3(x, y, z))
    //         .product()
    // }

    #[inline]
    pub(crate) fn eq_evals_at_primitive(r: &[F], mul_factor: &F, eq_evals: &mut [F]) {
        eq_evals[0] = *mul_factor;
        let mut cur_eval_num = 1;

        for r_i in r.iter() {
            let eq_z_i_zero = F::one() - r_i;
            let eq_z_i_one = r_i;
            for j in 0..cur_eval_num {
                eq_evals[j + cur_eval_num] = eq_evals[j] * eq_z_i_one;
                eq_evals[j] *= eq_z_i_zero;
            }
            cur_eval_num <<= 1;
        }
    }

    #[inline]
    pub(crate) fn eq_eval_at(
        r: &[F],
        mul_factor: &F,
        eq_evals: &mut [F],
        sqrt_n_1st: &mut [F],
        sqrt_n_2nd: &mut [F],
    ) {
        let first_half_bits = r.len() / 2;
        let first_half_mask = (1 << first_half_bits) - 1;
        Self::eq_evals_at_primitive(&r[0..first_half_bits], mul_factor, sqrt_n_1st);
        Self::eq_evals_at_primitive(&r[first_half_bits..], &F::one(), sqrt_n_2nd);

        for (i, eq_eval) in eq_evals.iter_mut().enumerate().take(1 << r.len()) {
            let first_half = i & first_half_mask;
            let second_half = i >> first_half_bits;
            *eq_eval = sqrt_n_1st[first_half] * sqrt_n_2nd[second_half];
        }
    }
}
