use arith::ExtensionField;
use gkr_engine::Transcript;
use polynomials::{MutableMultilinearExtension, UnivariatePoly};

#[allow(unused)]
pub(crate) fn vanilla_sumcheck_degree_2_mul_step_prove<F: ExtensionField>(
    poly0: &mut impl MutableMultilinearExtension<F>,
    poly1: &mut impl MutableMultilinearExtension<F>,
    fs_transcript: &mut impl Transcript<F>,
) -> (UnivariatePoly<F>, F, F) {
    assert_eq!(poly0.hypercube_size(), poly1.hypercube_size());

    let half_hypercube = poly0.hypercube_size() >> 1;
    let mut uni_evals = vec![F::ZERO; 3];

    (0..half_hypercube).for_each(|i| {
        let (p0v0, p1v0) = (poly0[i], poly1[i]);
        let (p0v1, p1v1) = (poly0[i + half_hypercube], poly1[i + half_hypercube]);
        let (p0v2, p1v2) = (p0v1.double() - p0v0, p1v1.double() - p1v0);

        let eval_at_0 = p0v0 * p1v0;
        uni_evals[0] += eval_at_0;

        let eval_at_1 = p0v1 * p1v1;
        uni_evals[1] += eval_at_1;

        let eval_at_2 = p0v2 * p1v2;
        uni_evals[2] += eval_at_2;
    });

    let round_univariate_poly = UnivariatePoly::from_evals(&uni_evals);
    round_univariate_poly
        .coeffs
        .iter()
        .for_each(|f| fs_transcript.append_field_element(f));

    let r_combine = fs_transcript.generate_challenge_field_element();
    poly0.fix_top_variable(r_combine);
    poly1.fix_top_variable(r_combine);

    let round_univariate_poly_at_r = round_univariate_poly.evaluate(r_combine);

    (round_univariate_poly, r_combine, round_univariate_poly_at_r)
}

#[allow(unused)]
pub(crate) fn vanilla_sumcheck_degree_2_mul_step_verify<F: ExtensionField>(
    prev_claim: &mut F,
    round_response: &UnivariatePoly<F>,
    fs_transcript: &mut impl Transcript<F>,
) -> Option<F> {
    if round_response.coeffs.len() != 3 {
        return None;
    }

    let presumed_prev_hypercube_sum =
        round_response.coeffs.iter().sum::<F>() + round_response.coeffs[0];
    if presumed_prev_hypercube_sum != *prev_claim {
        return None;
    }

    round_response
        .coeffs
        .iter()
        .for_each(|f| fs_transcript.append_field_element(f));

    let r_combine = fs_transcript.generate_challenge_field_element();
    *prev_claim = round_response.evaluate(r_combine);

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

        let mut p_claim = claim.clone();
        let sumcheck_resp: Vec<(UnivariatePoly<F>, F, F)> = (0..num_vars)
            .map(|_| {
                let (univ, r, next_claim) = vanilla_sumcheck_degree_2_mul_step_prove(
                    &mut multilinear_basis,
                    &mut eq_multilinear,
                    &mut fs_transcript_p,
                );

                p_claim = next_claim;

                (univ, r, next_claim)
            })
            .collect();

        let mut v_claim = claim.clone();
        sumcheck_resp
            .iter()
            .for_each(|(univ, expected_r, next_claim)| {
                let fold_var = vanilla_sumcheck_degree_2_mul_step_verify(
                    &mut v_claim,
                    univ,
                    &mut fs_transcript_v,
                );
                assert!(fold_var.is_some());

                assert_eq!(*expected_r, fold_var.unwrap());
                assert_eq!(v_claim, *next_claim);
            });
    }

    #[test]
    fn test_vanilla_sumcheck_degree_2_mul() {
        vanilla_sumcheck_degree_2_test_helper::<Fr>(15);
    }
}
