mod common;

use arith::{Field, Fr};
use ark_std::test_rng;
use gkr_engine::{
    BN254Config, ExpanderSingleVarChallenge, FieldEngine, MPIConfig, MPIEngine, Transcript,
};
use gkr_engine::{ExpanderPCS, StructuredReferenceString};
use gkr_hashers::Keccak256hasher;
use halo2curves::bn256::Bn256;
use poly_commit::HyperKZGPCS;
use polynomials::{MultiLinearPoly, MultilinearExtension};
use transcript::BytesHashTranscript;

const TEST_REPETITION: usize = 3;

fn test_hyperkzg_pcs_generics(num_vars_start: usize, num_vars_end: usize) {
    let mut rng = test_rng();

    (num_vars_start..=num_vars_end).for_each(|num_vars| {
        let xs: Vec<_> = (0..TEST_REPETITION)
            .map(|_| -> Vec<Fr> { (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect() })
            .collect();
        let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);

        common::test_pcs::<Fr, BytesHashTranscript<Keccak256hasher>, HyperKZGPCS<Bn256>>(
            &num_vars, &poly, &xs,
        );
    })
}

#[test]
fn test_hyperkzg_pcs_full_e2e() {
    test_hyperkzg_pcs_generics(2, 15)
}

fn test_hyper_bikzg_for_expander_gkr_generics(mpi_config_ref: &MPIConfig, total_num_vars: usize) {
    let mut rng = test_rng();

    // NOTE BN254 GKR SIMD pack size = 1, num vars in SIMD is 0
    let num_vars_in_mpi = mpi_config_ref.world_size().ilog2() as usize;
    let num_vars_in_each_poly = total_num_vars - num_vars_in_mpi;

    let global_poly = MultiLinearPoly::<Fr>::random(total_num_vars, &mut rng);
    let challenge_point = ExpanderSingleVarChallenge::<BN254Config> {
        r_mpi: (0..num_vars_in_mpi)
            .map(|_| Fr::random_unsafe(&mut rng))
            .collect(),
        r_simd: Vec::new(),
        rz: (0..num_vars_in_each_poly)
            .map(|_| Fr::random_unsafe(&mut rng))
            .collect(),
    };

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();

    // NOTE separate polynomial into different pieces by mpi rank
    let poly_vars_stride = (1 << global_poly.get_num_vars()) / mpi_config_ref.world_size();
    let poly_coeff_starts = mpi_config_ref.world_rank() * poly_vars_stride;
    let poly_coeff_ends = poly_coeff_starts + poly_vars_stride;
    let local_poly =
        MultiLinearPoly::new(global_poly.coeffs[poly_coeff_starts..poly_coeff_ends].to_vec());

    dbg!(local_poly.get_num_vars(), local_poly.coeffs[0]);

    let params = <HyperKZGPCS<Bn256> as ExpanderPCS<BN254Config, Fr>>::gen_params(
        num_vars_in_each_poly,
        mpi_config_ref.world_size(),
    );
    common::test_pcs_for_expander_gkr::<
        BN254Config,
        BytesHashTranscript<Keccak256hasher>,
        HyperKZGPCS<Bn256>,
    >(
        &params,
        mpi_config_ref,
        &mut transcript,
        &local_poly,
        &[challenge_point],
    );
}

#[test]
fn test_hyper_bikzg_for_expander_gkr() {
    let mpi_config = MPIConfig::prover_new();

    test_hyper_bikzg_for_expander_gkr_generics(&mpi_config, 1);
    test_hyper_bikzg_for_expander_gkr_generics(&mpi_config, 15);

    MPIConfig::finalize()
}

#[test]
fn test_kzg_batch_open() {
    common::test_batching::<Fr, BytesHashTranscript<Keccak256hasher>, HyperKZGPCS<Bn256>>();
    // common::test_batching_for_expander_gkr::<
    //     BN254Config,
    //     BytesHashTranscript<Keccak256hasher>,
    //     HyraxPCS<G1Affine>,
    // >();
}
