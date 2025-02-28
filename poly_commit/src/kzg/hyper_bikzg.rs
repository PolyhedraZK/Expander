// NOTE(HS): the algorithm port for HyperKZG to "HyperBiKZG" is sketched here:
// https://drive.google.com/file/d/1NcRnqdwFLcLi77DvSZH28QwslTuBVyb4/

use std::{io::Cursor, iter};

use arith::{ExtensionField, FieldSerde};
use halo2curves::{
    ff::Field,
    group::{prime::PrimeCurveAffine, Curve, GroupEncoding},
    pairing::MultiMillerLoop,
    CurveAffine,
};
use itertools::izip;
use mpi_config::MPIConfig;
use polynomials::MultilinearExtension;
use transcript::{transcript_root_broadcast, Transcript};

use crate::*;

pub fn coeff_form_hyper_bikzg_open<E, T>(
    srs: &CoefFormBiKZGLocalSRS<E>,
    mpi_config: &MPIConfig,
    coeffs: &impl MultilinearExtension<E::Fr>,
    local_alphas: &[E::Fr],
    mpi_alphas: &[E::Fr],
    fs_transcript: &mut T,
) where
    E: MultiMillerLoop + Default,
    T: Transcript<E::Fr>,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    //
    // Locally fold local variables, then commit to construct the poly oracles
    //

    let (local_folded_x_oracle_commits, local_folded_x_oracle_coeffs) =
        coeff_form_hyperkzg_local_poly_oracles(
            &srs.tau_x_srs,
            coeffs.hypercube_basis_ref(),
            local_alphas,
        );

    let local_final_eval_at_x = {
        let last_coeffs = local_folded_x_oracle_coeffs.last().unwrap().clone();
        let last_alpha = local_alphas[local_alphas.len() - 1];
        (E::Fr::ONE - last_alpha) * last_coeffs[0] + last_alpha * last_coeffs[1]
    };

    //
    // Leader party gathering evals and oracle commitments
    //

    let mut root_gathering_folded_oracle_commits: Vec<E::G1Affine> = Vec::new();
    let mut final_evals_at_x: Vec<E::Fr> = Vec::new();

    mpi_config.gather_vec(
        &local_folded_x_oracle_commits,
        &mut root_gathering_folded_oracle_commits,
    );
    mpi_config.gather_vec(&vec![local_final_eval_at_x], &mut final_evals_at_x);

    //
    // Leader party collect oracle commitments, sum them up for folded oracles
    //

    let mut folded_x_oracle_commits: Vec<E::G1Affine> = Vec::new();
    let mut y_oracle_commit: E::G1Affine = E::G1Affine::default();

    if mpi_config.is_root() {
        let g1_zero = E::G1Affine::default().to_curve();
        let mut folded_x_coms_g1 = vec![g1_zero; local_folded_x_oracle_commits.len()];

        root_gathering_folded_oracle_commits
            .chunks(local_folded_x_oracle_commits.len())
            .for_each(|folded_oracles| {
                izip!(&mut folded_x_coms_g1, folded_oracles)
                    .for_each(|(x_com_i, oracle_i)| *x_com_i += oracle_i.to_curve())
            });

        folded_x_oracle_commits = folded_x_coms_g1.iter().map(|f| f.to_affine()).collect();
        y_oracle_commit = coeff_form_uni_kzg_commit(&srs.tau_y_srs, &final_evals_at_x);
    }

    //
    // The leader party continues on folding over "final_evals" over only y variables.
    //

    let mut folded_y_oracle_commits: Vec<E::G1Affine> = Vec::new();
    let mut folded_y_oracle_coeffs: Vec<Vec<E::Fr>> = Vec::new();

    if mpi_config.is_root() {
        (folded_y_oracle_commits, folded_y_oracle_coeffs) =
            coeff_form_hyperkzg_local_poly_oracles(&srs.tau_y_srs, &final_evals_at_x, mpi_alphas);
    }

    //
    // The leader party feeds all folded oracles into RO, then sync party's transcript state
    //

    let mut folded_oracle_commitments: Vec<E::G1Affine> = Vec::new();

    if mpi_config.is_root() {
        folded_oracle_commitments = {
            let mut temp = folded_x_oracle_commits.clone();
            temp.push(y_oracle_commit);
            temp.extend_from_slice(&folded_y_oracle_commits);
            temp
        };

        folded_x_oracle_commits
            .iter()
            .chain(iter::once(&y_oracle_commit))
            .chain(&folded_y_oracle_commits)
            .for_each(|f| fs_transcript.append_u8_slice(f.to_bytes().as_ref()));
    }

    transcript_root_broadcast(fs_transcript, mpi_config);

    let beta_x = fs_transcript.generate_challenge_field_element();
    let beta_y = fs_transcript.generate_challenge_field_element();

    //
    // Local parties run HyperKZG evals at beta_x, -beta_x, beta_x^2 over folded coeffs
    //

    let local_folded_x_evals: HyperKZGLocalEvals<E> = coeff_form_hyperkzg_local_evals(
        coeffs.hypercube_basis_ref(),
        &local_folded_x_oracle_coeffs,
        local_alphas,
        beta_x,
    );

    let local_exported_folded_x_evals: HyperKZGExportedLocalEvals<E> =
        local_folded_x_evals.clone().into();

    //
    // Collect all exported local folded evals at x to the leader party
    //

    let mut root_gathering_exported_folded_x_evals: Vec<HyperKZGExportedLocalEvals<E>> = Vec::new();
    let mut root_aggregated_x_evals = HyperKZGAggregatedEvals::<E>::default();
    let mut root_folded_y_evals = HyperKZGLocalEvals::<E>::default();

    mpi_config.gather_vec(
        &vec![local_exported_folded_x_evals],
        &mut root_gathering_exported_folded_x_evals,
    );

    //
    // Leader aggregates all local exported evaluations (at x) by evaluating at y
    // by three points: beta_y, -beta_y, beta_y^2, then fold the final evals at x,
    // which is degree 0 for variable x, along variable y.
    //

    if mpi_config.is_root() {
        root_aggregated_x_evals = HyperKZGAggregatedEvals::new_from_exported_evals(
            &root_gathering_exported_folded_x_evals,
            beta_y,
        );

        root_folded_y_evals = coeff_form_hyperkzg_local_evals(
            &final_evals_at_x,
            &folded_y_oracle_coeffs,
            mpi_alphas,
            beta_y,
        );
    }

    //
    // The leader party feeds all evals into RO, then sync party's transcript state
    //

    if mpi_config.is_root() {
        root_aggregated_x_evals.append_to_transcript(fs_transcript);
        root_folded_y_evals.append_to_transcript(fs_transcript);
    }

    transcript_root_broadcast(fs_transcript, mpi_config);

    let gamma = fs_transcript.generate_challenge_field_element();

    //
    // The leader party linear combines folded coeffs at y with gamma,
    // then broadcast the coeffs back to local.
    //

    let mut leader_gamma_aggregated_y_coeffs: Vec<E::Fr> = Vec::new();

    if mpi_config.is_root() {
        leader_gamma_aggregated_y_coeffs = {
            let gamma_n = gamma.pow_vartime([local_alphas.len() as u64]);
            let mut temp = coeff_form_hyperkzg_local_oracle_polys_aggregate::<E>(
                &final_evals_at_x,
                &folded_y_oracle_coeffs,
                gamma,
            );
            temp.iter_mut().for_each(|t| *t *= gamma_n);
            temp
        };
    }

    // TODO(HS) can be improved to broadcast a vec, returning a coeff to each party
    {
        let mut serialized_y_coeffs: Vec<u8> = Vec::new();
        if mpi_config.is_root() {
            leader_gamma_aggregated_y_coeffs
                .serialize_into(&mut serialized_y_coeffs)
                .unwrap();
        }

        mpi_config.root_broadcast_bytes(&mut serialized_y_coeffs);
        leader_gamma_aggregated_y_coeffs = {
            let mut cursor = Cursor::new(serialized_y_coeffs);
            Vec::deserialize_from(&mut cursor).unwrap()
        };
    }

    //
    // Local party compute the linear combined folded coeffs at x with gamma,
    // then the degree2 Lagrange over beta_x, -beta_x, beta_x^2,
    // then vanish the local aggregated x coeffs at the three points above,
    // and commit to the final quotient poly
    //

    let mut local_gamma_aggregated_x_coeffs = {
        let mut f_gamma_local = coeff_form_hyperkzg_local_oracle_polys_aggregate::<E>(
            coeffs.hypercube_basis_ref(),
            &local_folded_x_oracle_coeffs,
            gamma,
        );

        f_gamma_local[0] += leader_gamma_aggregated_y_coeffs[mpi_config.world_rank()];
        f_gamma_local
    };

    let local_lagrange_degree2_at_x = {
        let mut local_degree_2 =
            local_folded_x_evals.interpolate_degree2_aggregated_evals(beta_x, gamma);

        local_degree_2[0] += leader_gamma_aggregated_y_coeffs[mpi_config.world_rank()];
        local_degree_2
    };

    let local_gamma_aggregated_x_quotient = {
        let mut nom = local_gamma_aggregated_x_coeffs.clone();
        polynomial_add(&mut nom, -E::Fr::ONE, &local_lagrange_degree2_at_x);
        univariate_roots_quotient(nom, &[beta_x, -beta_x, beta_x * beta_x])
    };

    let local_gamma_aggregated_x_quotient_commitment_g1: E::G1 =
        coeff_form_uni_kzg_commit(&srs.tau_x_srs, &local_gamma_aggregated_x_quotient).to_curve();

    //
    // Leader collect all the quotient commitment at x, sum it up and feed it to RO,
    // then sync transcript state
    //

    let mut root_gathering_gamma_aggregated_x_quotient_commitment_g1s: Vec<E::G1> = Vec::new();
    mpi_config.gather_vec(
        &vec![local_gamma_aggregated_x_quotient_commitment_g1],
        &mut root_gathering_gamma_aggregated_x_quotient_commitment_g1s,
    );

    let mut gamma_aggregated_x_quotient_commitment: E::G1Affine = E::G1Affine::default();

    if mpi_config.is_root() {
        gamma_aggregated_x_quotient_commitment =
            root_gathering_gamma_aggregated_x_quotient_commitment_g1s
                .iter()
                .sum::<E::G1>()
                .to_affine();

        fs_transcript.append_u8_slice(gamma_aggregated_x_quotient_commitment.to_bytes().as_ref());
    }

    transcript_root_broadcast(fs_transcript, mpi_config);

    let delta_x = fs_transcript.generate_challenge_field_element();

    //
    // Locally compute the Lagrange-degree2 interpolation at delta_x, pool at leader
    //

    let mut degree2_evals_at_delta_x: Vec<E::Fr> = Vec::new();

    let local_degree2_eval_at_delta_x = local_lagrange_degree2_at_x[0]
        + local_lagrange_degree2_at_x[1] * delta_x
        + local_lagrange_degree2_at_x[2] * delta_x * delta_x;

    mpi_config.gather_vec(
        &vec![local_degree2_eval_at_delta_x],
        &mut degree2_evals_at_delta_x,
    );

    //
    // Leader does similar thing - quotient at beta_y, -beta_y, beta_y^2,
    // commit the quotient polynomial commitment at y, feed it to RO,
    // then sync transcript state
    //

    let mut leader_quotient_y_coeffs: Vec<E::Fr> = Vec::new();
    #[allow(unused)]
    let mut leader_quotient_y_commitment: E::G1Affine = E::G1Affine::default();

    if mpi_config.is_root() {
        let num_y_coeffs = mpi_config.world_size();

        // NOTE(HS) interpolate at beta_y, beta_y2, -beta_y on lagrange_degree2_delta_x
        let lagrange_degree2_delta_y = {
            let pos_beta_y_pow_series = powers_series(&beta_y, num_y_coeffs);
            let neg_beta_y_pow_series = powers_series(&(-beta_y), num_y_coeffs);
            let beta_y2_pow_series = powers_series(&(beta_y * beta_y), num_y_coeffs);

            let at_beta_y = univariate_evaluate(&degree2_evals_at_delta_x, &pos_beta_y_pow_series);
            let at_neg_beta_y =
                univariate_evaluate(&degree2_evals_at_delta_x, &neg_beta_y_pow_series);
            let at_beta_y2 = univariate_evaluate(&degree2_evals_at_delta_x, &beta_y2_pow_series);

            coeff_form_degree2_lagrange(
                [beta_y, -beta_y, beta_y * beta_y],
                [at_beta_y, at_neg_beta_y, at_beta_y2],
            )
        };

        leader_quotient_y_coeffs = {
            let mut nom = degree2_evals_at_delta_x.clone();
            polynomial_add(&mut nom, -E::Fr::ONE, &lagrange_degree2_delta_y);
            univariate_roots_quotient(nom, &[beta_y, -beta_y, beta_y * beta_y])
        };
        leader_quotient_y_coeffs.resize(num_y_coeffs, E::Fr::ZERO);

        leader_quotient_y_commitment =
            coeff_form_uni_kzg_commit(&srs.tau_y_srs, &leader_quotient_y_coeffs);

        fs_transcript.append_u8_slice(leader_quotient_y_commitment.to_bytes().as_ref());
    }

    transcript_root_broadcast(fs_transcript, mpi_config);

    let delta_y = fs_transcript.generate_challenge_field_element();

    //
    // Leader send out the quotient on y coefficients back to local parties
    //

    // TODO(HS) can be better if the root only send corresponding coeffs to the parties
    {
        let mut serialized_y_quotient_coeffs: Vec<u8> = Vec::new();
        if mpi_config.is_root() {
            leader_quotient_y_coeffs
                .serialize_into(&mut serialized_y_quotient_coeffs)
                .unwrap();
        }

        mpi_config.root_broadcast_bytes(&mut serialized_y_quotient_coeffs);
        leader_quotient_y_coeffs = {
            let mut cursor = Cursor::new(serialized_y_quotient_coeffs);
            Vec::deserialize_from(&mut cursor).unwrap()
        };
        leader_quotient_y_coeffs.resize(mpi_config.world_size(), E::Fr::ZERO);
    }

    //
    // Final step for local - trip off the prior quotients at x and y on \pm beta and beta^2
    //

    // NOTE(HS) f_gamma_s - (delta_x - beta_x) ... (delta_x - beta_x2) f_gamma_quotient_s
    //                    - (delta_y - beta_y) ... (delta_y - beta_y2) lagrange_quotient_y
    let delta_x_denom = (delta_x - beta_x) * (delta_x - beta_x * beta_x) * (delta_x + beta_x);
    let delta_y_denom = (delta_y - beta_y) * (delta_y - beta_y * beta_y) * (delta_y + beta_y);

    polynomial_add(
        &mut local_gamma_aggregated_x_coeffs,
        -delta_x_denom,
        &local_gamma_aggregated_x_quotient,
    );
    local_gamma_aggregated_x_coeffs[0] -=
        delta_y_denom * leader_quotient_y_coeffs[mpi_config.world_rank()];

    //
    // BiKZG commit to the last bivariate poly
    //

    let mut gathered_eval_opens: Vec<(E::Fr, E::G1Affine)> = Vec::new();
    let local_eval_open =
        coeff_form_uni_kzg_open_eval(&srs.tau_x_srs, &local_gamma_aggregated_x_coeffs, delta_x);

    mpi_config.gather_vec(&vec![local_eval_open], &mut gathered_eval_opens);

    // TODO(HS) root broadcast the proof out and everyone have a copy can just verify
    let mut _final_opening = BiKZGProof::<E>::default();
    let mut _hyper_bikzg_opening = HyperBiKZGOpening::<E>::default();

    if mpi_config.is_root() {
        (_, _final_opening) = coeff_form_bi_kzg_open_leader(srs, &gathered_eval_opens, delta_y);

        _hyper_bikzg_opening = HyperBiKZGOpening {
            folded_oracle_commitments,
            aggregated_evals: root_aggregated_x_evals,
            leader_evals: root_folded_y_evals.into(),
            beta_x_commitment: gamma_aggregated_x_quotient_commitment,
            beta_y_commitment: leader_quotient_y_commitment,
            quotient_delta_x_commitment: _final_opening.quotient_x,
            quotient_delta_y_commitment: _final_opening.quotient_y,
        };
    }
}
