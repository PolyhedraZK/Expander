//! Multi-points batch opening
use arith::{ExtensionField, Field};
use ark_std::log2;
use gkr_engine::Transcript;
use halo2curves::group::Curve;
use halo2curves::{ff::PrimeField, CurveAffine};
use polynomials::MultiLinearPoly;
use polynomials::{EqPolynomial, MultilinearExtension};
use serdes::ExpSerde;
use sumcheck::{IOPProof, SumCheck, SumOfProductsPoly};

/// Merge a list of polynomials and its corresponding points into a single polynomial
/// Returns
/// - the new point for evaluation
/// - the new polynomial that is merged via sumcheck
/// - the proof of the sumcheck
#[allow(clippy::type_complexity)]
pub fn prover_merge_points<C>(
    polys: &[MultiLinearPoly<C::Scalar>],
    points: &[Vec<C::Scalar>],
    transcript: &mut impl Transcript,
) -> (
    Vec<C::Scalar>,
    MultiLinearPoly<C::Scalar>,
    IOPProof<C::Scalar>,
)
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let num_vars = polys[0].num_vars();
    let k = polys.len();
    let ell = log2(k) as usize;

    // challenge point t
    let t = transcript.generate_field_elements::<C::Scalar>(ell);

    // eq(t, i) for i in [0..k]
    let eq_t_i = EqPolynomial::build_eq_x_r(&t);

    // \tilde g_i(b) = eq(t, i) * f_i(b)
    let mut tilde_gs = vec![];
    for (index, f_i) in polys.iter().enumerate() {
        let mut tilde_g_eval = vec![C::Scalar::zero(); 1 << num_vars];
        for (j, &f_i_eval) in f_i.coeffs.iter().enumerate() {
            tilde_g_eval[j] = f_i_eval * eq_t_i[index];
        }
        tilde_gs.push(MultiLinearPoly {
            coeffs: tilde_g_eval,
        });
    }

    // built the virtual polynomial for SumCheck
    let tilde_eqs: Vec<MultiLinearPoly<C::Scalar>> = points
        .iter()
        .map(|point| {
            let eq_b_zi = EqPolynomial::build_eq_x_r(point);
            MultiLinearPoly { coeffs: eq_b_zi }
        })
        .collect();

    let mut sumcheck_poly = SumOfProductsPoly::new();
    for (tilde_g, tilde_eq) in tilde_gs.iter().zip(tilde_eqs.into_iter()) {
        sumcheck_poly.add_pair(tilde_g.clone(), tilde_eq);
    }

    let proof = SumCheck::<C::Scalar>::prove(&sumcheck_poly, transcript);

    let a2 = proof.export_point_to_expander();

    // build g'(X) = \sum_i=1..k \tilde eq_i(a2) * \tilde g_i(X) where (a2) is the
    // sumcheck's point \tilde eq_i(a2) = eq(a2, point_i)
    let mut g_prime_evals = vec![C::Scalar::zero(); 1 << num_vars];

    for (tilde_g, point) in tilde_gs.iter().zip(points.iter()) {
        let eq_i_a2 = EqPolynomial::eq_vec(a2.as_ref(), point);
        for (j, &tilde_g_eval) in tilde_g.coeffs.iter().enumerate() {
            g_prime_evals[j] += tilde_g_eval * eq_i_a2;
        }
    }
    let g_prime = MultiLinearPoly {
        coeffs: g_prime_evals,
    };

    (a2, g_prime, proof)
}

pub fn verifier_merge_points<C>(
    commitments: &[impl AsRef<[C]>],
    points: &[Vec<C::Scalar>],
    values: &[C::Scalar],
    sumcheck_proof: &IOPProof<C::Scalar>,
    transcript: &mut impl Transcript,
) -> (C::Scalar, Vec<C>)
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    let k = commitments.len();
    let ell = log2(k) as usize;
    let num_var = sumcheck_proof.point.len();

    // sum check point (a2)
    let a2 = sumcheck_proof.export_point_to_expander();

    // challenge point t
    let t = transcript.generate_field_elements::<C::Scalar>(ell);

    let eq_t_i = EqPolynomial::build_eq_x_r(&t);

    // build g' commitment

    // todo: use MSM
    // let mut scalars = vec![];
    // let mut bases = vec![];

    let mut g_prime_commit_elems = vec![C::Curve::default(); commitments[0].as_ref().len()];
    for (i, point) in points.iter().enumerate() {
        let eq_i_a2 = EqPolynomial::eq_vec(a2.as_ref(), point);
        let scalar = eq_i_a2 * eq_t_i[i];
        for (j, &base) in commitments[i].as_ref().iter().enumerate() {
            g_prime_commit_elems[j] += base * scalar;
        }
    }
    let mut g_prime_commit_affine = vec![C::default(); commitments[0].as_ref().len()];
    C::Curve::batch_normalize(&g_prime_commit_elems, &mut g_prime_commit_affine);

    // ensure \sum_i eq(t, <i>) * f_i_evals matches the sum via SumCheck
    let mut sum = C::Scalar::zero();
    for (i, &e) in eq_t_i.iter().enumerate().take(k) {
        sum += e * values[i];
    }

    let subclaim = SumCheck::<C::Scalar>::verify(sum, sumcheck_proof, num_var, transcript);

    let tilde_g_eval = subclaim.expected_evaluation;

    (tilde_g_eval, g_prime_commit_affine)
}
