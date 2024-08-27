use arith::{Field, BinomialExtensionField, GF2_128x8, M31Ext3, M31Ext3x16, GF2_128};
use ark_std::test_rng;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use tynm::type_name;

fn random_element<F: Field>() -> F {
    let mut rng = test_rng();
    F::random_unsafe(&mut rng)
}

pub(crate) fn bench_field<F: Field+BinomialExtensionField>(c: &mut Criterion) {
    c.bench_function(
        &format!(
            "mul-by-base-throughput<{}> 100x times {}x ",
            type_name::<F>(),
            F::SIZE * 8 / F::FIELD_SIZE
        ),
        |b| {
            b.iter_batched(
                || {
                    (
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F::BaseField>(),
                        random_element::<F::BaseField>(),
                        random_element::<F::BaseField>(),
                        random_element::<F::BaseField>(),
                    )
                },
                |(mut x, mut y, mut z, mut w, xx, yy, zz, ww)| {
                    for _ in 0..25 {
                        (x, y, z, w) = (x.mul_by_base_field(&xx), y.mul_by_base_field(&yy), z.mul_by_base_field(&zz), w.mul_by_base_field(&ww));
                    }
                    (x, y, z, w)
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "mul-by-base-latency<{}> 100x times {}x ",
            type_name::<F>(),
            F::SIZE * 8 / F::FIELD_SIZE
        ),
        |b| {
            b.iter_batched(
                || {
                    (
                        random_element::<F>(),
                        random_element::<F::BaseField>(),
                    )
                },
                |(mut x, xx)| {
                    for _ in 0..100 {
                        x = x.mul_by_base_field(&xx);
                    }
                    x
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "add-by-base-throughput<{}> 100x times {}x ",
            type_name::<F>(),
            F::SIZE * 8 / F::FIELD_SIZE
        ),
        |b| {
            b.iter_batched(
                || {
                    (
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F::BaseField>(),
                        random_element::<F::BaseField>(),
                        random_element::<F::BaseField>(),
                        random_element::<F::BaseField>(),
                    )
                },
                |(mut x, mut y, mut z, mut w, xx, yy, zz, ww)| {
                    for _ in 0..25 {
                        (x, y, z, w) = (x.add_by_base_field(&xx), y.add_by_base_field(&yy), z.add_by_base_field(&zz), w.add_by_base_field(&ww));
                    }
                    (x, y, z, w)
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "add-by-base-latency<{}> 100x times {}x ",
            type_name::<F>(),
            F::SIZE * 8 / F::FIELD_SIZE
        ),
        |b| {
            b.iter_batched(
                || {
                    (
                        random_element::<F>(),
                        random_element::<F::BaseField>(),
                    )
                },
                |(mut x, xx)| {
                    for _ in 0..100 {
                        x = x.add_by_base_field(&xx);
                    }
                    x
                },
                BatchSize::SmallInput,
            )
        },
    );

}

fn ext_by_base_benchmark(c: &mut Criterion) {
    bench_field::<M31Ext3>(c);
    bench_field::<M31Ext3x16>(c);
    bench_field::<GF2_128>(c);
    bench_field::<GF2_128x8>(c);
}

criterion_group!(ext_by_base_benches, ext_by_base_benchmark);
criterion_main!(ext_by_base_benches);
