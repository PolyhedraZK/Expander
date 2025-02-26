mod common;

use arith::{BN254Fr, Field};
use ark_std::test_rng;
use gkr_field_config::BN254Config;
use halo2curves::bn256::G1Affine;
use mpi_config::MPIConfig;
use poly_commit::{ExpanderGKRChallenge, HyraxPCS};
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher, Transcript};

const TEST_REPETITION: usize = 3;

fn test_hyrax_pcs_generics(num_vars_start: usize, num_vars_end: usize) {
    let mut rng = test_rng();

    (num_vars_start..=num_vars_end).for_each(|num_vars| {
        let xs: Vec<_> = (0..TEST_REPETITION)
            .map(|_| -> Vec<BN254Fr> {
                (0..num_vars)
                    .map(|_| BN254Fr::random_unsafe(&mut rng))
                    .collect()
            })
            .collect();
        let poly = MultiLinearPoly::<BN254Fr>::random(num_vars, &mut rng);

        common::test_pcs::<
            BN254Fr,
            BytesHashTranscript<BN254Fr, Keccak256hasher>,
            HyraxPCS<G1Affine, BytesHashTranscript<BN254Fr, Keccak256hasher>>,
        >(&num_vars, &poly, &xs);
    })
}

#[test]
fn test_hyrax_pcs_e2e() {
    test_hyrax_pcs_generics(3, 17)
}

fn test_hyrax_for_expander_gkr_generics(mpi_config_ref: &MPIConfig, total_num_vars: usize) {
    let mut rng = test_rng();

    // NOTE BN254 GKR SIMD pack size = 1, num vars in SIMD is 0
    let num_vars_in_mpi = mpi_config_ref.world_size().ilog2() as usize;
    let num_vars_in_each_poly = total_num_vars - num_vars_in_mpi;

    let global_poly = MultiLinearPoly::<BN254Fr>::random(total_num_vars, &mut rng);
    let challenge_point = ExpanderGKRChallenge::<BN254Config> {
        x_mpi: (0..num_vars_in_mpi)
            .map(|_| BN254Fr::random_unsafe(&mut rng))
            .collect(),
        x_simd: Vec::new(),
        x: (0..num_vars_in_each_poly)
            .map(|_| BN254Fr::random_unsafe(&mut rng))
            .collect(),
    };

    let mut transcript = BytesHashTranscript::<BN254Fr, Keccak256hasher>::new();

    // NOTE separate polynomial into different pieces by mpi rank
    let poly_vars_stride = (1 << global_poly.get_num_vars()) / mpi_config_ref.world_size();
    let poly_coeff_starts = mpi_config_ref.world_rank() * poly_vars_stride;
    let poly_coeff_ends = poly_coeff_starts + poly_vars_stride;
    let local_poly =
        MultiLinearPoly::new(global_poly.coeffs[poly_coeff_starts..poly_coeff_ends].to_vec());

    dbg!(local_poly.get_num_vars(), local_poly.coeffs[0]);

    common::test_pcs_for_expander_gkr::<
        BN254Config,
        BytesHashTranscript<BN254Fr, Keccak256hasher>,
        HyraxPCS<G1Affine, BytesHashTranscript<BN254Fr, Keccak256hasher>>,
    >(
        &num_vars_in_each_poly,
        mpi_config_ref,
        &mut transcript,
        &local_poly,
        &[challenge_point],
    );
}

#[test]
fn test_hyrax_for_expander_gkr() {
    let mpi_config = MPIConfig::new();

    test_hyrax_for_expander_gkr_generics(&mpi_config, 19);

    MPIConfig::finalize()
}
