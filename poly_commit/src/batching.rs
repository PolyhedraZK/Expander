//! Multi-points batch opening
//! Uses Rayon to parallelize the computation.
use arith::{ExtensionField, Field};
use ark_ec::{AffineRepr, CurveGroup, VariableBaseMSM};
use ark_ff::{Field as ArkField, PrimeField, Zero};
use ark_std::log2;
use gkr_engine::Transcript;
use polynomials::{EqPolynomial, MultilinearExtension};
use polynomials::{MultiLinearPoly, SumOfProductsPoly};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serdes::ExpSerde;
use sumcheck::{IOPProof, SumCheck};
use utils::timer::Timer;

/// Merge a list of polynomials and its corresponding points into a single polynomial
/// Returns
/// - the new point for evaluation
/// - the new polynomial that is merged via sumcheck
/// - the proof of the sumcheck
#[allow(clippy::type_complexity)]
pub fn prover_merge_points<C>(
    polys: &[impl MultilinearExtension<C::ScalarField>],
    points: &[impl AsRef<[C::ScalarField]>],
    transcript: &mut impl Transcript,
) -> (
    Vec<C::ScalarField>,
    MultiLinearPoly<C::ScalarField>,
    IOPProof<C::ScalarField>,
)
where
    C: AffineRepr + ExpSerde,
    C::ScalarField: ExtensionField + PrimeField,
{
    let num_vars = polys.iter().map(|p| p.num_vars()).max().unwrap_or(0);
    let k = polys.len();
    let ell = log2(k) as usize;

    // challenge point t
    let t = transcript.generate_field_elements::<C::ScalarField>(ell);

    // eq(t, i) for i in [0..k]
    let eq_t_i = EqPolynomial::build_eq_x_r(&t);

    // \tilde g_i(b) = eq(t, i) * f_i(b)
    let timer = Timer::new("Building tilde g_i(b)", true);

    let tilde_gs: Vec<MultiLinearPoly<C::ScalarField>> = polys
        .par_iter()
        .enumerate()
        .map(|(index, f_i)| {
            let mut tilde_g_eval = vec![<C::ScalarField as Zero>::zero(); 1 << f_i.num_vars()];
            for (j, &f_i_eval) in f_i.hypercube_basis_ref().iter().enumerate() {
                tilde_g_eval[j] = f_i_eval * eq_t_i[index];
            }

            MultiLinearPoly {
                coeffs: tilde_g_eval,
            }
        })
        .collect::<Vec<_>>();
    timer.stop();

    // built the virtual polynomial for SumCheck
    let timer = Timer::new("Building tilde eqs", true);
    let points = points.iter().map(|p| p.as_ref()).collect::<Vec<_>>();
    let tilde_eqs: Vec<MultiLinearPoly<C::ScalarField>> = points
        .par_iter()
        .map(|point| {
            let mut eq_b_zi = vec![<C::ScalarField as Zero>::zero(); 1 << point.len()];
            EqPolynomial::build_eq_x_r_with_buf(
                point,
                &<C::ScalarField as ArkField>::ONE,
                &mut eq_b_zi,
            );
            MultiLinearPoly { coeffs: eq_b_zi }
        })
        .collect();
    timer.stop();

    let timer = Timer::new("Sumcheck merging points", true);
    let mut sumcheck_poly = SumOfProductsPoly::new();
    for (tilde_g, tilde_eq) in tilde_gs.iter().zip(tilde_eqs.into_iter()) {
        sumcheck_poly.add_pair(tilde_g.clone(), tilde_eq);
    }
    let proof = SumCheck::<C::ScalarField>::prove(&sumcheck_poly, transcript);
    timer.stop();

    let a2 = proof.export_point_to_expander();

    // build g'(X) = \sum_i=1..k \tilde eq_i(a2) * \tilde g_i(X) where (a2) is the
    // sumcheck's point \tilde eq_i(a2) = eq(a2, point_i)
    let timer = Timer::new("Building g'(X)", true);

    let mut g_prime_evals: Vec<C::ScalarField> =
        vec![<C::ScalarField as Zero>::zero(); 1 << num_vars];
    let eq_i_a2_polys: Vec<C::ScalarField> = points
        .par_iter()
        .map(|point| {
            let mut padded_point = point.to_vec();
            padded_point.resize(num_vars, <C::ScalarField as Zero>::zero());
            EqPolynomial::eq_vec(a2.as_ref(), &padded_point)
        })
        .collect::<Vec<_>>();

    for (tilde_g, &eq_i_a2) in tilde_gs.iter().zip(eq_i_a2_polys.iter()) {
        for (j, &tilde_g_eval) in tilde_g.coeffs.iter().enumerate() {
            g_prime_evals[j] += tilde_g_eval * eq_i_a2;
        }
    }
    let g_prime = MultiLinearPoly {
        coeffs: g_prime_evals,
    };
    timer.stop();
    (a2, g_prime, proof)
}

pub fn verifier_merge_points<C>(
    commitments: &[impl AsRef<[C::Affine]>],
    points: &[impl AsRef<[C::ScalarField]>],
    values: &[C::ScalarField],
    sumcheck_proof: &IOPProof<C::ScalarField>,
    transcript: &mut impl Transcript,
) -> (bool, C::ScalarField, Vec<C::Affine>)
where
    // C: ExpSerde +
    C: CurveGroup,
    C::ScalarField: ExtensionField + PrimeField,
{
    let (padded_commitments, padded_points) = pad_commitments_and_points::<C>(commitments, points);

    let k = padded_commitments.len();
    let ell = log2(k) as usize;
    let num_var = sumcheck_proof.point.len();
    assert!(
        num_var == padded_points[0].len(),
        "Number of variables in sumcheck proof must match the number of variables in points"
    );

    // sum check point (a2)
    let a2 = sumcheck_proof.export_point_to_expander();

    // challenge point t
    let t = transcript.generate_field_elements::<C::ScalarField>(ell);

    let eq_t_i = EqPolynomial::build_eq_x_r(&t);

    // build g' commitment
    // let bases: Vec<C::Affine> = padded_commitments
    //     .iter()
    //     .map(|c| c.as_ref())
    //     .collect::<Vec<_>>();
    let bases_transposed = transpose::<C::Affine>(padded_commitments.as_slice());

    let scalars = padded_points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let eq_i_a2 = EqPolynomial::eq_vec(a2.as_ref(), p);
            eq_i_a2 * eq_t_i[i]
        })
        .collect::<Vec<C::ScalarField>>();

    let g_prime_commit_elems = bases_transposed
        .iter()
        .map(|base| VariableBaseMSM::msm(base, &scalars).unwrap())
        .collect::<Vec<_>>();

    let g_prime_commit_affine = C::normalize_batch(&g_prime_commit_elems);

    // ensure \sum_i eq(t, <i>) * f_i_evals matches the sum via SumCheck
    let mut sum = <C::ScalarField as Zero>::zero();
    for (i, &e) in eq_t_i.iter().enumerate().take(k) {
        sum += e * values[i];
    }

    let (verified, subclaim) =
        SumCheck::<C::ScalarField>::verify(sum, sumcheck_proof, num_var, transcript);

    let tilde_g_eval = subclaim.expected_evaluation;

    (verified, tilde_g_eval, g_prime_commit_affine)
}

#[inline]
fn transpose<C: Clone + Copy>(m: &[Vec<C>]) -> Vec<Vec<C>> {
    if m.is_empty() || m[0].is_empty() {
        return Vec::new();
    }

    let rows = m.len();
    let cols = m[0].len();

    let mut transposed = vec![Vec::with_capacity(rows); cols];

    for row in m.iter() {
        for j in 0..cols {
            transposed[j].push(row[j]);
        }
    }

    transposed
}

#[inline]
#[allow(clippy::type_complexity)]
#[allow(unused)]
fn pad_polynomials_and_points<C>(
    polys: &[impl MultilinearExtension<C::ScalarField>],
    points: &[impl AsRef<[C::ScalarField]>],
) -> (
    Vec<MultiLinearPoly<C::ScalarField>>,
    Vec<Vec<C::ScalarField>>,
)
where
    C: CurveGroup,
    C::ScalarField: Field,
{
    let max_size = polys
        .iter()
        .map(|p| p.hypercube_basis_ref().len())
        .max()
        .unwrap_or(0);
    let max_num_vars = log2(max_size) as usize;
    let padded_polys = polys
        .iter()
        .map(|poly| {
            let mut coeffs = poly.hypercube_basis_ref().to_vec();
            coeffs.resize(max_size, <C::ScalarField as Field>::zero());
            MultiLinearPoly { coeffs }
        })
        .collect::<Vec<_>>();
    let padded_points = points
        .iter()
        .map(|point| {
            let mut padded_point = point.as_ref().to_vec();
            padded_point.resize(max_num_vars, <C::ScalarField as Zero>::zero());
            padded_point
        })
        .collect::<Vec<_>>();

    (padded_polys, padded_points)
}

#[inline]
// Each commitment is a vector of curve points
// This generalizes both KZG and Hyrax commitments
fn pad_commitments_and_points<C>(
    commitments: &[impl AsRef<[C::Affine]>],
    points: &[impl AsRef<[C::ScalarField]>],
) -> (Vec<Vec<C::Affine>>, Vec<Vec<C::ScalarField>>)
where
    C: CurveGroup,
{
    let max_num_vars = points.iter().map(|p| p.as_ref().len()).max().unwrap_or(0);
    let max_commit_size = commitments
        .iter()
        .map(|c| c.as_ref().len())
        .max()
        .unwrap_or(0);

    let padded_points = points
        .iter()
        .map(|point| {
            let mut padded_point = point.as_ref().to_vec();
            padded_point.resize(max_num_vars, <C::ScalarField as Zero>::zero());
            padded_point
        })
        .collect::<Vec<_>>();

    let padded_commitments = commitments
        .iter()
        .map(|commitment| {
            let mut padded_commitment = commitment.as_ref().to_vec();
            padded_commitment.resize(max_commit_size, C::Affine::zero());
            padded_commitment
        })
        .collect::<Vec<_>>();

    (padded_commitments, padded_points)
}
