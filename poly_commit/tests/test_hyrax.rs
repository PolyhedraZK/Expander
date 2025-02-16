mod common;

use arith::{BN254Fr, Field};
use ark_std::test_rng;
use halo2curves::bn256::G1Affine;
use poly_commit::HyraxPCS;
use polynomials::MultiLinearPoly;
use transcript::{BytesHashTranscript, Keccak256hasher};

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
