use arith::ExtensionField;
use gkr_engine::Transcript;
use polynomials::{MutableMultilinearExtension, UnivariatePoly};

#[allow(unused)]
pub(crate) fn vanilla_sumcheck_degree_2_mul_step_prove<F: ExtensionField>(
    poly0: &mut impl MutableMultilinearExtension<F>,
    poly1: &mut impl MutableMultilinearExtension<F>,
    fs_transcript: &mut impl Transcript<F>,
) -> (UnivariatePoly<F>, F) {
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

    let uni_poly = {
        let a = at_0;
        let a_plus_c = (at_pos_1 + at_neg_1) * F::INV_2;
        let c = a_plus_c - a;
        let b = at_pos_1 - a - c;

        UnivariatePoly::new(vec![a, b, c])
    };

    uni_poly
        .coeffs
        .iter()
        .for_each(|f| fs_transcript.append_field_element(f));

    let r_combine = fs_transcript.generate_challenge_field_element();
    poly0.fix_top_variable(r_combine);
    poly1.fix_top_variable(r_combine);

    (uni_poly, r_combine)
}

#[allow(unused)]
pub(crate) fn vanilla_sumcheck_degree_2_mul_step_verify<F: ExtensionField>(
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
mod vanilla_sumcheck_degree_2_test {
    use arith::{ExtensionField, Fr};
    use ark_std::test_rng;
    use gkr_engine::Transcript;
    use gkr_hashers::Keccak256hasher;
    use polynomials::{EqPolynomial, MultiLinearPoly, UnivariatePoly};
    use transcript::BytesHashTranscript;

    use crate::fri::vanilla_sumcheck::{
        vanilla_sumcheck_degree_2_mul_step_prove, vanilla_sumcheck_degree_2_mul_step_verify,
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
                vanilla_sumcheck_degree_2_mul_step_prove(
                    &mut multilinear_basis,
                    &mut eq_multilinear,
                    &mut fs_transcript_p,
                )
            })
            .collect();

        let mut v_claim = claim.clone();
        sumcheck_resp.iter().for_each(|(univ, expected_r)| {
            let fold_var =
                vanilla_sumcheck_degree_2_mul_step_verify(&mut v_claim, univ, &mut fs_transcript_v);
            assert!(fold_var.is_some());

            assert_eq!(*expected_r, fold_var.unwrap());
        });
    }

    #[test]
    fn test_vanilla_sumcheck_degree_2_mul() {
        vanilla_sumcheck_degree_2_test_helper::<Fr>(15);
    }
}
