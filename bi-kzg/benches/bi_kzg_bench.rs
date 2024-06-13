use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::short_weierstrass::Projective;
use ark_ec::Group;
use ark_ec::{AffineRepr, VariableBaseMSM};
use ark_ff::Zero;
use ark_ff::{PrimeField, UniformRand};
use halo2curves::bn256;
use halo2curves::ff::Field;
use halo2curves::msm::best_multiexp;
use rand::Rng;
use std::{thread, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let degree_set = vec![2048, 4096, 8192, 16384];
    //let degree_set = vec![32786];
    // let mut res = vec![];
    for degree in degree_set {
        let mut rng = rand::thread_rng();

        let affine_bases = (0..degree)
            .map(|_| G1Affine::rand(&mut rng))
            .collect::<Vec<_>>();
        let proj_bases = affine_bases
            .iter()
            .map(|g| g.into_group())
            .collect::<Vec<_>>();
        let scalars = (0..degree).map(|_| Fr::rand(&mut rng)).collect::<Vec<_>>();

        c.bench_function(&format!("arkworks MSM with degree {}", degree), |b| {
            b.iter(|| {
                _ = G1Projective::msm(&affine_bases, &scalars);
            });
        });

        let cpu_set = vec![4, 8, 16, 32];
        for nb_cpu in cpu_set.iter() {
            c.bench_function(
                &format!("custimzied MSM with degree {}, cpu={}", degree, nb_cpu),
                |b| {
                    b.iter(|| {
                        _ = quick_exp(&proj_bases, &scalars, *nb_cpu);
                    });
                },
            );
        }

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

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(2)).warm_up_time(Duration::from_secs(1));
    targets = criterion_benchmark
}
criterion_main!(benches);

pub fn quick_exp(points: &Vec<G1Projective>, scalars: &Vec<Fr>, num_cpu: usize) -> G1Projective {
    let c = best_c(points.len(), num_cpu);
    assert!(64 % c == 0);

    let mut result = G1Projective::zero();
    let nb_fr_bits = nb_fr_bits();

    let point_len = points.len();
    let num_points_per_cpu = (point_len + num_cpu - 1) / num_cpu;

    let mut handles = vec![];

    let point_ptr_usize = points.as_ptr() as usize;
    let scalar_ptr_usize = scalars.as_ptr() as usize;
    for i in 0..num_cpu {
        let handle = thread::spawn(move || {
            let mut bucket = vec![G1Projective::zero(); (nb_fr_bits / c) * (1 << c)];
            let start = i * num_points_per_cpu;
            let end = std::cmp::min((i + 1) * num_points_per_cpu, point_len);
            let point_ptr = point_ptr_usize as *const G1Projective;
            let scalar_ptr = scalar_ptr_usize as *const Fr;
            for j in start..end {
                let point = unsafe { point_ptr.add(j).read() };
                let scalar_bigint = unsafe { scalar_ptr.add(j).read() };
                let limbs = scalar_bigint.into_bigint().0;
                for k in 0..(nb_fr_bits / c) {
                    let mut temp = limbs[(k * c) / 64];
                    let inside_idx = (k * c) % 64;
                    temp >>= inside_idx;
                    temp &= (1 << c) - 1;
                    bucket[(k << c) + temp as usize] += point;
                }
            }
            let mut partial_sum = G1Projective::zero();
            for i in (0..(nb_fr_bits / c)).rev() {
                for _ in 0..c {
                    partial_sum.double_in_place();
                }
                let mut tmp = G1Projective::zero();
                for j in 0..(1 << c) {
                    let mut pow_res = G1Projective::zero();
                    let mut base = bucket[(i << c) + j];
                    for k in 0..c {
                        if (j >> k) & 1 == 1 {
                            pow_res += base;
                        }
                        base.double_in_place();
                    }
                    tmp += pow_res;
                }
                partial_sum += tmp;
            }
            partial_sum
        });
        handles.push(handle);
    }
    for handle in handles {
        let partial_sum = handle.join().unwrap();
        result += partial_sum;
    }
    result
}

#[inline]
fn nb_fr_bits() -> usize {
    256
}

#[inline]
fn cost_c(len: usize, c: usize, num_cpu: usize) -> usize {
    let mut sum = 0_usize;
    //first part
    sum += (nb_fr_bits() / c) * len / num_cpu;
    //second part
    sum += nb_fr_bits() * (1 << c);
    sum
}

#[inline]
fn best_c(len: usize, num_cpu: usize) -> usize {
    if cost_c(len, 8, num_cpu) > cost_c(len, 16, num_cpu) {
        16
    } else {
        8
    }
}
