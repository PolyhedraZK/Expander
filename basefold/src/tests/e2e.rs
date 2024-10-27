use std::time::Instant;

use arith::Field;
use ark_std::test_rng;
use babybear::{BabyBear, BabyBearExt3};
use polynomials::MultiLinearPoly;
use rand::RngCore;
use transcript::{BytesHashTranscript, Keccak256hasher, SHA256hasher, Transcript};

use crate::{BaseFoldPCS, BasefoldParam, PolynomialCommitmentScheme};

#[test]
fn test_basefold_vanilla() {
    let mut rng = test_rng();

    for i in 5..=6 {
        for _ in 0..10 {
            test_basefold_helper(i, &mut rng);
        }
    }
}

fn test_basefold_helper(num_vars: usize, mut rng: impl RngCore) {
    let pp = BasefoldParam::<
    BytesHashTranscript<BabyBear, Keccak256hasher>, BabyBearExt3, BabyBear,
>::new(3);

    let poly = MultiLinearPoly::<BabyBear>::random(num_vars, &mut rng);

    let opening_point: Vec<_> = (0..num_vars)
        .map(|_| BabyBear::random_unsafe(&mut rng))
        .collect();
    let eval = poly.evaluate(&opening_point);

    let now = Instant::now();
    let commitment = BaseFoldPCS::commit(&pp, &poly);
    println!("committing elapsed {}", now.elapsed().as_millis());

    let mut prover_transcript = BytesHashTranscript::<BabyBear, Keccak256hasher>::new();
    let mut verifier_transcript = BytesHashTranscript::<BabyBear, Keccak256hasher>::new();

    let now = Instant::now();
    let eval_proof = BaseFoldPCS::open(
        &pp,
        &commitment,
        &poly,
        &opening_point,
        &mut prover_transcript,
    );
    println!("proving elapsed {}", now.elapsed().as_millis());

    let now = Instant::now();
    let verify = BaseFoldPCS::verify(
        &pp,
        &commitment,
        &opening_point,
        &eval,
        &eval_proof,
        &mut verifier_transcript,
    );
    assert!(verify, "failed to verify");
    println!("verifying elapsed {}", now.elapsed().as_millis());
}
