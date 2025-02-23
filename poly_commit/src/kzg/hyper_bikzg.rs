// NOTE(HS): the algorithm port for HyperKZG to "HyperBiKZG" is sketched here:
// https://drive.google.com/file/d/1NcRnqdwFLcLi77DvSZH28QwslTuBVyb4/

use std::iter;

use arith::ExtensionField;
use halo2curves::{
    ff::Field,
    group::{prime::PrimeCurveAffine, GroupEncoding},
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
    E: MultiMillerLoop,
    T: Transcript<E::Fr>,
    E::G1Affine: CurveAffine<ScalarExt = E::Fr, CurveExt = E::G1>,
    E::Fr: ExtensionField,
{
    let (folded_oracle_commits, folded_oracle_coeffs) = coeff_form_hyperkzg_local_poly_oracles(
        &srs.tau_x_srs,
        coeffs.hypercube_basis_ref(),
        local_alphas,
    );

    let final_local_eval = {
        let last_coeffs = folded_oracle_coeffs[folded_oracle_coeffs.len() - 1].clone();
        let last_alpha = local_alphas[local_alphas.len() - 1];
        last_coeffs[0] * (E::Fr::ONE - last_alpha) * last_coeffs[0] + last_alpha * last_coeffs[1]
    };

    let mut root_gathering_folded_oracle_commits = Vec::new();
    let mut final_evals = Vec::new();
    mpi_config.gather_vec(
        &folded_oracle_commits,
        &mut root_gathering_folded_oracle_commits,
    );
    mpi_config.gather_vec(&vec![final_local_eval], &mut final_evals);

    if mpi_config.is_root() {
        let g1_zero = E::G1Affine::default().to_curve();
        let mut folded_x_coms_g1 = vec![g1_zero; folded_oracle_commits.len()];

        root_gathering_folded_oracle_commits
            .chunks(folded_oracle_commits.len())
            .for_each(|folded_oracles| {
                izip!(&mut folded_x_coms_g1, folded_oracles)
                    .for_each(|(x_com_i, oracle_i)| *x_com_i += oracle_i.to_curve())
            });

        let folded_x_coms: Vec<E::G1Affine> = folded_x_coms_g1
            .iter()
            .map(|f| Into::<E::G1Affine>::into(*f))
            .collect();

        let folded_y_oracle = coeff_form_uni_kzg_commit(&srs.tau_y_srs, &final_evals);
        let (folded_mpi_oracle_coms, folded_mpi_oracle_coeffs_s) =
            coeff_form_hyperkzg_local_poly_oracles(&srs.tau_y_srs, &final_evals, mpi_alphas);

        folded_x_coms
            .iter()
            .chain(iter::once(&folded_y_oracle))
            .chain(&folded_mpi_oracle_coms)
            .for_each(|f| fs_transcript.append_u8_slice(f.to_bytes().as_ref()));
    }

    transcript_root_broadcast(fs_transcript, mpi_config);

    let beta_x = fs_transcript.generate_challenge_field_element();
    let beta_y = fs_transcript.generate_challenge_field_element();
}
