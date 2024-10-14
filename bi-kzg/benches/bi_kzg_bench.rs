use bi_kzg::{BiKZGVerifierParam, BivariatePolynomial, CoeffFormBiKZG, PolynomialCommitmentScheme};
use halo2curves::bn256::{self, Bn256, Fr};
use halo2curves::ff::Field;
use halo2curves::msm::best_multiexp;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_msm(c: &mut Criterion) {
    let degree_set = vec![2048, 4096, 8192, 16384];
    for degree in degree_set {
        let mut rng = rand::thread_rng();

        let affine_bases = (0..degree)
            .map(|_| bn256::G1Affine::random(&mut rng))
            .collect::<Vec<_>>();

        let scalars = (0..degree)
            .map(|_| bn256::Fr::random(&mut rng))
            .collect::<Vec<_>>();
        c.bench_function(&format!("halo2 MSM with degree {}", degree), |b| {
            b.iter(|| {
                _ = best_multiexp(&scalars, &affine_bases);
            });
        });
    }
}

fn bench_commit(c: &mut Criterion) {
    let degree_set = [4usize, 16, 64, 256];
    let mut rng = rand::thread_rng();
    for &degree_0 in degree_set.iter() {
        for &degree_1 in degree_set.iter() {
            let srs = CoeffFormBiKZG::<Bn256>::gen_srs_for_testing(&mut rng, degree_0, degree_1);
            let vk = BiKZGVerifierParam::<Bn256>::from(&srs);
            let poly = BivariatePolynomial::<Fr>::random(&mut rng, degree_0, degree_1);
            c.bench_function(
                &format!("bi-kzg commit with degrees {} {}", degree_0, degree_1),
                |b| {
                    b.iter(|| {
                        let _ = black_box(CoeffFormBiKZG::<Bn256>::commit(&srs, &poly));
                    });
                },
            );

            let com = CoeffFormBiKZG::<Bn256>::commit(&srs, &poly);

            let points = (0..10)
                .map(|_| (Fr::random(&mut rng), Fr::random(&mut rng)))
                .collect::<Vec<_>>();
            c.bench_function(
                &format!(
                    "bi-kzg open {} points with degrees {} {}",
                    points.len(),
                    degree_0,
                    degree_1
                ),
                |b| {
                    b.iter(|| {
                        let _ = points
                            .iter()
                            .map(|p| CoeffFormBiKZG::<Bn256>::open(&srs, &poly, p))
                            .collect::<Vec<_>>();
                    });
                },
            );
            let proofs = points
                .iter()
                .map(|p| CoeffFormBiKZG::<Bn256>::open(&srs, &poly, p))
                .collect::<Vec<_>>();

            c.bench_function(
                &format!(
                    "bi-kzg verify {} points with degrees {} {}",
                    points.len(),
                    degree_0,
                    degree_1
                ),
                |b| {
                    b.iter(|| {
                        points.iter().zip(proofs.iter()).for_each(|(p, proof)| {
                            assert!(CoeffFormBiKZG::<Bn256>::verify(
                                &vk, &com, p, &proof.1, &proof.0
                            ))
                        })
                    });
                },
            );
        }
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(2)).warm_up_time(Duration::from_secs(1));
    targets = bench_msm, bench_commit
}
criterion_main!(benches);
