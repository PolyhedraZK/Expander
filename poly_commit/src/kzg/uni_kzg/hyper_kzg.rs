use std::iter;

use ::utils::timer::Timer;
use arith::{ExtensionField, Field};
use ark_ec::pairing::Pairing;
use gkr_engine::Transcript;
use itertools::izip;
use polynomials::MultilinearExtension;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serdes::ExpSerde;

use crate::{
    batching::{prover_merge_points, verifier_merge_points},
    traits::BatchOpening,
    *,
};

#[inline(always)]
pub(crate) fn coeff_form_hyperkzg_local_poly_oracles<E>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::ScalarField],
    local_alphas: &[E::ScalarField],
) -> (Vec<E::G1Affine>, Vec<Vec<E::ScalarField>>)
where
    E: Pairing,
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
    E::ScalarField: ExtensionField,
{
    let mut local_coeffs = coeffs.to_vec();

    local_alphas[..local_alphas.len() - 1]
        .iter()
        .map(|alpha| {
            local_coeffs = local_coeffs
                .chunks(2)
                .map(|c| (E::ScalarField::one() - alpha) * c[0] + *alpha * c[1])
                .collect();

            let folded_oracle_commit = coeff_form_uni_kzg_commit(srs, &local_coeffs);

            (folded_oracle_commit, local_coeffs.clone())
        })
        .unzip()
}

#[inline(always)]
pub(crate) fn coeff_form_hyperkzg_local_evals<E>(
    coeffs: &[E::ScalarField],
    folded_oracle_coeffs: &[Vec<E::ScalarField>],
    local_alphas: &[E::ScalarField],
    beta: E::ScalarField,
) -> HyperKZGLocalEvals<E>
where
    E: Pairing,
    E::ScalarField: ExtensionField,
{
    let beta2 = beta * beta;
    let beta_inv = beta.inv().unwrap();
    let two_inv = E::ScalarField::one().double().inv().unwrap();
    let beta_pow_series = powers_series(&beta, coeffs.len());
    let neg_beta_pow_series = powers_series(&(-beta), coeffs.len());

    let beta2_eval = {
        let beta2_pow_series = powers_series(&beta2, coeffs.len());
        univariate_evaluate(coeffs, &beta2_pow_series)
    };

    let mut local_evals = HyperKZGLocalEvals::<E>::new_from_beta2_evals(beta2_eval);

    izip!(
        iter::once(coeffs).chain(folded_oracle_coeffs.iter().map(|x| x.as_slice())),
        local_alphas
    )
    .for_each(|(cs, alpha)| {
        let beta_eval = univariate_evaluate(cs, &beta_pow_series);
        let neg_beta_eval = univariate_evaluate(cs, &neg_beta_pow_series);

        let beta2_eval = two_inv
            * ((beta_eval + neg_beta_eval) * (E::ScalarField::one() - alpha)
                + (beta_eval - neg_beta_eval) * beta_inv * alpha);

        local_evals.beta2_evals.push(beta2_eval);
        local_evals.pos_beta_evals.push(beta_eval);
        local_evals.neg_beta_evals.push(neg_beta_eval);
    });

    local_evals
}

#[inline(always)]
pub(crate) fn coeff_form_hyperkzg_local_oracle_polys_aggregate<E>(
    coeffs: &[E::ScalarField],
    folded_oracle_coeffs: &[Vec<E::ScalarField>],
    gamma: E::ScalarField,
) -> Vec<E::ScalarField>
where
    E: Pairing,
    E::ScalarField: ExtensionField,
{
    let gamma_pow_series = powers_series(&gamma, folded_oracle_coeffs.len() + 1);
    let mut f = coeffs.to_vec();
    izip!(&gamma_pow_series[1..], folded_oracle_coeffs)
        .for_each(|(gamma_i, folded_f)| polynomial_add(&mut f, *gamma_i, folded_f));
    f
}

#[inline(always)]
pub fn coeff_form_uni_hyperkzg_open<E, T>(
    srs: &CoefFormUniKZGSRS<E>,
    coeffs: &[E::ScalarField],
    alphas: &[E::ScalarField],
    fs_transcript: &mut T,
) -> (E::ScalarField, HyperUniKZGOpening<E>)
where
    E: Pairing,
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
    E::ScalarField: ExtensionField,
    T: Transcript,
{
    let (folded_oracle_commitments, folded_oracle_coeffs) =
        coeff_form_hyperkzg_local_poly_oracles(srs, coeffs, alphas);

    folded_oracle_commitments.iter().for_each(|f| {
        fs_transcript.append_serializable_data(f);
    });

    let beta = fs_transcript.generate_field_element::<E::ScalarField>();
    let beta2 = beta * beta;

    let local_evals =
        coeff_form_hyperkzg_local_evals::<E>(coeffs, &folded_oracle_coeffs, alphas, beta);
    local_evals.append_to_transcript(fs_transcript);

    let gamma = fs_transcript.generate_field_element::<E::ScalarField>();

    let mut f_gamma =
        coeff_form_hyperkzg_local_oracle_polys_aggregate::<E>(coeffs, &folded_oracle_coeffs, gamma);
    let lagrange_degree2 = local_evals.interpolate_degree2_aggregated_evals(beta, gamma);
    let f_gamma_quotient = {
        let mut nom = f_gamma.clone();
        polynomial_add(&mut nom, -E::ScalarField::one(), &lagrange_degree2);
        univariate_roots_quotient(nom, &[beta, beta2, -beta])
    };
    let beta_x_commitment = coeff_form_uni_kzg_commit(srs, &f_gamma_quotient);

    fs_transcript.append_serializable_data(&beta_x_commitment);

    let tau = fs_transcript.generate_field_element::<E::ScalarField>();
    let vanishing_at_tau = {
        let f_gamma_denom = (tau - beta) * (tau + beta) * (tau - beta2);
        let lagrange_degree2_at_tau =
            lagrange_degree2[0] + lagrange_degree2[1] * tau + lagrange_degree2[2] * tau * tau;

        polynomial_add(&mut f_gamma, -f_gamma_denom, &f_gamma_quotient);
        let (coeffs, remainder) = univariate_degree_one_quotient(&f_gamma, tau);
        assert_eq!(lagrange_degree2_at_tau, remainder);
        coeffs
    };
    let quotient_delta_x_commitment = coeff_form_uni_kzg_commit(srs, &vanishing_at_tau);

    (
        local_evals.multilinear_final_eval(),
        HyperUniKZGOpening {
            folded_oracle_commitments,
            evals_at_x: local_evals.into(),
            beta_x_commitment,
            quotient_delta_x_commitment,
        },
    )
}

#[inline(always)]
pub fn coeff_form_uni_hyperkzg_verify<E, T>(
    vk: &UniKZGVerifierParams<E>,
    comm: E::G1Affine,
    alphas: &[E::ScalarField],
    eval: E::ScalarField,
    opening: &HyperUniKZGOpening<E>,
    fs_transcript: &mut T,
) -> bool
where
    E: Pairing,
    E::G1Affine: ExpSerde,
    E::G2Affine: ExpSerde,
    E::ScalarField: ExtensionField + ExpSerde,
    T: Transcript,
{
    opening
        .folded_oracle_commitments
        .iter()
        .for_each(|f| fs_transcript.append_serializable_data(f));

    let beta = fs_transcript.generate_field_element::<E::ScalarField>();
    let beta2 = beta * beta;

    let local_evals =
        HyperKZGLocalEvals::<E>::new_from_exported_evals(&opening.evals_at_x, alphas, beta);

    opening.evals_at_x.append_to_transcript(fs_transcript);

    if local_evals.multilinear_final_eval() != eval {
        return false;
    }

    let gamma = fs_transcript.generate_field_element::<E::ScalarField>();
    let gamma_pow_series = powers_series(&gamma, alphas.len());
    let v_beta = univariate_evaluate(&local_evals.pos_beta_evals, &gamma_pow_series);
    let v_neg_beta = univariate_evaluate(&local_evals.neg_beta_evals, &gamma_pow_series);
    let v_beta2 = univariate_evaluate(&local_evals.beta2_evals, &gamma_pow_series);
    let lagrange_degree2 =
        coeff_form_degree2_lagrange([beta, -beta, beta2], [v_beta, v_neg_beta, v_beta2]);

    let folded_g1_oracle_comms: Vec<E::G1> = opening
        .folded_oracle_commitments
        .iter()
        .map(|c| E::G1::from(*c))
        .collect();
    let commitment_agg_g1: E::G1 =
        comm + univariate_evaluate(&folded_g1_oracle_comms, &gamma_pow_series[1..]);

    fs_transcript.append_serializable_data(&opening.beta_x_commitment);
    let tau = fs_transcript.generate_field_element::<E::ScalarField>();

    let q_weight = (tau - beta) * (tau - beta2) * (tau + beta);
    let lagrange_eval =
        lagrange_degree2[0] + lagrange_degree2[1] * tau + lagrange_degree2[2] * tau * tau;

    coeff_form_uni_kzg_verify(
        vk,
        (commitment_agg_g1 - opening.beta_x_commitment * q_weight).into(),
        tau,
        lagrange_eval,
        opening.quotient_delta_x_commitment,
    );

    true
}

pub fn multiple_points_batch_open_impl<E, PCS>(
    proving_key: &CoefFormUniKZGSRS<E>,
    polys: &[impl MultilinearExtension<E::ScalarField>],
    points: &[impl AsRef<[E::ScalarField]>],
    transcript: &mut impl Transcript,
) -> (Vec<E::ScalarField>, BatchOpening<E::ScalarField, PCS>)
where
    E: Pairing,
    E::ScalarField: ExtensionField,
    E::G1Affine: ExpSerde + Default,
    E::G2Affine: ExpSerde + Default,
    PCS: PolynomialCommitmentScheme<E::ScalarField, Opening = HyperUniKZGOpening<E>>,
{
    let timer = Timer::new("batch_opening", true);
    // generate evals for each polynomial at its corresponding point
    let eval_timer = Timer::new("eval all polys", true);
    let points = points.iter().map(|p| p.as_ref()).collect::<Vec<_>>();
    let evals: Vec<E::ScalarField> = polys
        .par_iter()
        .zip_eq(points.par_iter())
        .map(|(poly, point)| poly.evaluate(point))
        .collect();
    eval_timer.stop();

    let merger_timer = Timer::new("merging points", true);
    let (new_point, g_prime, proof) =
        prover_merge_points::<E::G1Affine>(polys, &points, transcript);
    merger_timer.stop();

    let pcs_timer = Timer::new("kzg_open", true);
    let (_g_prime_eval, g_prime_proof) =
        coeff_form_uni_hyperkzg_open(proving_key, &g_prime.coeffs, &new_point, transcript);
    pcs_timer.stop();

    timer.stop();
    (
        evals,
        BatchOpening {
            sum_check_proof: proof,
            g_prime_proof,
        },
    )
}

pub fn multiple_points_batch_verify_impl<E, PCS>(
    verifying_key: &UniKZGVerifierParams<E>,
    commitments: &[impl AsRef<UniKZGCommitment<E>>],
    points: &[impl AsRef<[E::ScalarField]>],
    values: &[E::ScalarField],
    batch_opening: &BatchOpening<E::ScalarField, PCS>,
    transcript: &mut impl Transcript,
) -> bool
where
    E: Pairing,
    E::ScalarField: ExtensionField,
    E::G1Affine: ExpSerde + Default,
    E::G2Affine: ExpSerde + Default,
    PCS: PolynomialCommitmentScheme<E::ScalarField, Opening = HyperUniKZGOpening<E>>,
{
    let a2 = batch_opening.sum_check_proof.export_point_to_expander();

    let commitments = commitments
        .iter()
        .map(|c| vec![c.as_ref().0])
        .collect::<Vec<_>>();

    let (verified, tilde_g_eval, g_prime_commit) = verifier_merge_points::<E::G1>(
        &commitments,
        points,
        values,
        &batch_opening.sum_check_proof,
        transcript,
    );

    if !verified {
        return false;
    }

    // verify commitment
    coeff_form_uni_hyperkzg_verify(
        verifying_key,
        g_prime_commit[0],
        a2.as_ref(),
        tilde_g_eval,
        &batch_opening.g_prime_proof,
        transcript,
    )
}
