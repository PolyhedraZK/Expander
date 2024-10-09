use arith::Field;
use ark_std::test_rng;
use babybear::{BabyBear, BabyBearExt3};
use mpoly::MultiLinearPoly;
use rand::RngCore;
use transcript::{SHA256hasher, Transcript, TranscriptInstance};

use crate::BasefoldParam;

#[test]
fn test_basefold_vanilla() {
    let mut rng = test_rng();

    for i in 5..=18 {
        for _ in 0..10 {
            test_basefold_helper(i, &mut rng);
        }
    }
}

fn test_basefold_helper(num_vars: usize, mut rng: impl RngCore) {
    let pp = BasefoldParam::<
    TranscriptInstance<SHA256hasher>, SHA256hasher, BabyBearExt3, BabyBear,
>::new(3);

    let poly = MultiLinearPoly::<BabyBear>::random(num_vars, &mut rng);

    let opening_point: Vec<_> = (0..num_vars)
        .map(|_| BabyBear::random_unsafe(&mut rng))
        .collect();
    let eval = poly.evaluate(&opening_point);

    // let now = Instant::now();
    // let commitment = BasefoldCommitmentScheme::commit(&poly, &pp);
    // println!("committing elapsed {}", now.elapsed().as_millis());

    // let mut prover_transcript = ProofTranscript::new(b"example");
    // let mut verifier_transcript = ProofTranscript::new(b"example");

    // let now = Instant::now();
    // let eval_proof = BasefoldCommitmentScheme::prove(
    //     &poly,
    //     &pp,
    //     &commitment,
    //     &opening_point,
    //     &mut prover_transcript,
    // );
    // println!("proving elapsed {}", now.elapsed().as_millis());

    // let now = Instant::now();
    // BasefoldCommitmentScheme::verify(
    //     &eval_proof,
    //     &pp,
    //     &mut verifier_transcript,
    //     &opening_point,
    //     &eval,
    //     &commitment,
    // )
    // .unwrap();
    // println!("verifying elapsed {}", now.elapsed().as_millis());
}
