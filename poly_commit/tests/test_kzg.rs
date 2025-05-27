mod common;

use arith::{Field, Fr};
use ark_std::test_rng;
use gkr_engine::{
    BN254Config, DeferredCheck, ExpanderSingleVarChallenge, FieldEngine, MPIConfig, MPIEngine,
    Transcript,
};
use gkr_engine::{ExpanderPCS, StructuredReferenceString};
use gkr_hashers::Keccak256hasher;
use halo2curves::bn256::Bn256;
use poly_commit::{HyperKZGPCS, PairingAccumulator};
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
fn test_hyper_bikzg_batch_verification() {
    let mut rng = test_rng();
    let mpi_config = MPIConfig::prover_new();

    for num_vars in 2..=15 {
        let srs = <HyperKZGPCS<Bn256> as ExpanderPCS<BN254Config, Fr>>::gen_srs_for_testing(
            &num_vars,
            &mpi_config,
            &mut rng,
        );
        let (proving_key, verification_key) = srs.into_keys();

        let transcript = BytesHashTranscript::<Keccak256hasher>::new();
        let mut scratch_pad =
            <HyperKZGPCS<Bn256> as ExpanderPCS<BN254Config, Fr>>::init_scratch_pad(
                &num_vars,
                &mpi_config,
            );
        let mut pairing_accumulator = PairingAccumulator::<Bn256>::default();

        let polys = (0..3).map(|_| MultiLinearPoly::<Fr>::random(num_vars, &mut rng));

        for poly in polys {
            let mut rng = test_rng();
            let challenge_point_1 = ExpanderSingleVarChallenge::<BN254Config> {
                r_mpi: Vec::new(),
                r_simd: Vec::new(),
                rz: (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect(),
            };
            let challenge_point_2 = ExpanderSingleVarChallenge::<BN254Config> {
                r_mpi: Vec::new(),
                r_simd: Vec::new(),
                rz: (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect(),
            };
            let commitment = <HyperKZGPCS<Bn256> as ExpanderPCS<BN254Config, Fr>>::commit(
                &num_vars,
                &mpi_config,
                &proving_key,
                &poly,
                &mut scratch_pad,
            )
            .unwrap();

            for c in [challenge_point_1, challenge_point_2] {
                // open
                let mut local_transcript = transcript.clone();
                let opening = <HyperKZGPCS<Bn256> as ExpanderPCS<BN254Config, Fr>>::open(
                    &num_vars,
                    &mpi_config,
                    &proving_key,
                    &poly,
                    &c,
                    &mut local_transcript,
                    &mut scratch_pad,
                )
                .unwrap();

                // partial verify
                let mut local_transcript = transcript.clone();
                let v = BN254Config::single_core_eval_circuit_vals_at_expander_challenge(
                    &poly.hypercube_basis_ref(),
                    &c,
                );

                let partial_verify =
                    <HyperKZGPCS<Bn256> as ExpanderPCS<BN254Config, Fr>>::partial_verify(
                        &num_vars,
                        &verification_key,
                        &commitment,
                        &c,
                        v,
                        &mut local_transcript,
                        &opening,
                        &mut pairing_accumulator,
                    );

                assert!(partial_verify);
            }
        }
        // finalize the pairing check
        assert!(pairing_accumulator.final_check());
    }
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
