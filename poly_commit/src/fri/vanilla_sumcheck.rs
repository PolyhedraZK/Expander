use arith::ExtensionField;
use gkr_engine::Transcript;
use polynomials::{MultilinearExtension, MutableMultilinearExtension, UnivariatePoly};

#[inline(always)]
pub(crate) fn sumcheck_step_prove_gen_challenge<F: ExtensionField>(
    poly0: &impl MultilinearExtension<F>,
    poly1: &impl MultilinearExtension<F>,
) -> [F; 3] {
    assert_eq!(poly0.hypercube_size(), poly1.hypercube_size());

    let half_hypercube = poly0.hypercube_size() >> 1;

    let (mut at_0, mut at_pos_1, mut at_neg_1) = (F::ZERO, F::ZERO, F::ZERO);

    (0..half_hypercube).for_each(|i| {
        let (p0v0, p1v0) = (poly0[i], poly1[i]);
        let (p0_at_pos1, p1_at_pos1) = (poly0[i + half_hypercube], poly1[i + half_hypercube]);
        let (p0_at_neg1, p1_at_neg1) = (p0v0.double() - p0_at_pos1, p1v0.double() - p1_at_pos1);

        at_0 += p0v0 * p1v0;
        at_pos_1 += p0_at_pos1 * p1_at_pos1;
        at_neg_1 += p0_at_neg1 * p1_at_neg1;
    });

    {
        let a = at_0;
        let a_plus_c = (at_pos_1 + at_neg_1) * F::INV_2;
        let c = a_plus_c - a;
        let b = at_pos_1 - a - c;

        [a, b, c]
    }
}

#[inline(always)]
pub(crate) fn sumcheck_step_prove_receive_challenge<F: ExtensionField>(
    poly0: &mut impl MutableMultilinearExtension<F>,
    poly1: &mut impl MutableMultilinearExtension<F>,
    r_combine: F,
) {
    poly0.fix_top_variable(r_combine);
    poly1.fix_top_variable(r_combine);
}

#[inline(always)]
pub(crate) fn sumcheck_deg_2_mul_step_verify<F: ExtensionField>(
    claim: &mut F,
    uni_poly: &UnivariatePoly<F>,
    fs_transcript: &mut impl Transcript<F>,
) -> Option<F> {
    if uni_poly.coeffs.len() != 3 {
        return None;
    }

    let presumed_prev_claim = uni_poly.coeffs.iter().sum::<F>() + uni_poly.coeffs[0];
    if presumed_prev_claim != *claim {
        return None;
    }

    uni_poly
        .coeffs
        .iter()
        .for_each(|f| fs_transcript.append_field_element(f));

    let r_combine = fs_transcript.generate_challenge_field_element();
    *claim = uni_poly.evaluate(r_combine);

    Some(r_combine)
}

#[cfg(test)]
mod vanilla_sumcheck_test {
    use arith::ExtensionField;
    use ark_std::test_rng;
    use gkr_engine::Transcript;
    use gkr_hashers::Keccak256hasher;
    use goldilocks::GoldilocksExt2;
    use mersenne31::{M31Ext3, M31Ext6};
    use polynomials::{EqPolynomial, MultiLinearPoly, UnivariatePoly};
    use transcript::BytesHashTranscript;

    use crate::fri::vanilla_sumcheck::{
        sumcheck_deg_2_mul_step_verify, sumcheck_step_prove_gen_challenge,
        sumcheck_step_prove_receive_challenge,
    };

    fn vanilla_sumcheck_degree_2_test_helper<F: ExtensionField>(num_vars: usize) {
        let mut rng = test_rng();

        let mut multilinear_basis = MultiLinearPoly::<F>::random(num_vars, &mut rng);
        let point: Vec<_> = (0..num_vars).map(|_| F::random_unsafe(&mut rng)).collect();
        let claim = multilinear_basis.evaluate_jolt(&point);

        let mut eq_multilinear = MultiLinearPoly::new(EqPolynomial::build_eq_x_r(&point));

        let mut fs_transcript_p = BytesHashTranscript::<F, Keccak256hasher>::new();
        let mut fs_transcript_v = fs_transcript_p.clone();

        let sumcheck_resp: Vec<(UnivariatePoly<F>, F)> = (0..num_vars)
            .map(|_| {
                let coeffs = sumcheck_step_prove_gen_challenge(&multilinear_basis, &eq_multilinear);
                let r = {
                    coeffs
                        .iter()
                        .for_each(|c| fs_transcript_p.append_field_element(c));
                    fs_transcript_p.generate_challenge_field_element()
                };
                sumcheck_step_prove_receive_challenge(
                    &mut multilinear_basis,
                    &mut eq_multilinear,
                    r,
                );
                (UnivariatePoly::new(coeffs.to_vec()), r)
            })
            .collect();

        let mut v_claim = claim.clone();
        sumcheck_resp.iter().for_each(|(univ, expected_r)| {
            let r_i = sumcheck_deg_2_mul_step_verify(&mut v_claim, univ, &mut fs_transcript_v);
            assert!(r_i.is_some());

            assert_eq!(*expected_r, r_i.unwrap());
        });
    }

    #[test]
    fn test_vanilla_sumcheck_degree_2_mul() {
        vanilla_sumcheck_degree_2_test_helper::<M31Ext3>(19);
        vanilla_sumcheck_degree_2_test_helper::<M31Ext6>(19);
        vanilla_sumcheck_degree_2_test_helper::<GoldilocksExt2>(19);
    }
}
