use ark_std::test_rng;
use criterion::{BatchSize, Criterion};
use tynm::type_name;

use crate::{ExtensionField, Field};

fn random_element<F: Field>() -> F {
    let mut rng = test_rng();
    F::random_unsafe(&mut rng)
}

pub fn bench_ext_field<F: Field + ExtensionField>(c: &mut Criterion) {
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
                        (x, y, z, w) = (
                            x.mul_by_base_field(&xx),
                            y.mul_by_base_field(&yy),
                            z.mul_by_base_field(&zz),
                            w.mul_by_base_field(&ww),
                        );
                    }
                    (x, y, z, w)
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "mul-by-x-throughput<{}> 100x times {}x ",
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
                    )
                },
                |(mut x, mut y, mut z, mut w)| {
                    for _ in 0..25 {
                        (x, y, z, w) = (x.mul_by_x(), y.mul_by_x(), z.mul_by_x(), w.mul_by_x());
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
                || (random_element::<F>(), random_element::<F::BaseField>()),
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
                        (x, y, z, w) = (
                            x.add_by_base_field(&xx),
                            y.add_by_base_field(&yy),
                            z.add_by_base_field(&zz),
                            w.add_by_base_field(&ww),
                        );
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
                || (random_element::<F>(), random_element::<F::BaseField>()),
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

pub fn bench_field<F: Field>(c: &mut Criterion) {
    c.bench_function(
        &format!(
            "mul-throughput<{}> 100x times {}x ",
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
                    )
                },
                |(mut x, mut y, mut z, mut w)| {
                    for _ in 0..25 {
                        (x, y, z, w) = (x * y, y * z, z * w, w * x);
                    }
                    (x, y, z, w)
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "mul-latency<{}> 100x times {}x ",
            type_name::<F>(),
            F::SIZE * 8 / F::FIELD_SIZE
        ),
        |b| {
            b.iter_batched(
                || random_element::<F>(),
                |mut x| {
                    for _ in 0..100 {
                        x = x * x;
                    }
                    x
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "sqr-throughput<{}> 100x times {}x",
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
                    )
                },
                |(mut x, mut y, mut z, mut w)| {
                    for _ in 0..25 {
                        (x, y, z, w) = (x.square(), y.square(), z.square(), w.square());
                    }
                    (x, y, z, w)
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "sqr-latency<{}> 100x times {}x",
            type_name::<F>(),
            F::SIZE * 8 / F::FIELD_SIZE
        ),
        |b| {
            b.iter_batched(
                || random_element::<F>(),
                |mut x| {
                    for _ in 0..100 {
                        x = x.square();
                    }
                    x
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "add-throughput<{}> 100x times {}x",
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
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                        random_element::<F>(),
                    )
                },
                |(mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h, mut i, mut j)| {
                    for _ in 0..10 {
                        (a, b, c, d, e, f, g, h, i, j) = (
                            a + b,
                            b + c,
                            c + d,
                            d + e,
                            e + f,
                            f + g,
                            g + h,
                            h + i,
                            i + j,
                            j + a,
                        );
                    }
                    (a, b, c, d, e, f, g, h, i, j)
                },
                BatchSize::SmallInput,
            )
        },
    );

    c.bench_function(
        &format!(
            "add-latency<{}> 100x times {}x",
            type_name::<F>(),
            F::SIZE * 8 / F::FIELD_SIZE
        ),
        |b| {
            b.iter_batched(
                || random_element::<F>(),
                |mut x| {
                    for _ in 0..100 {
                        x = x + x;
                    }
                    x
                },
                BatchSize::SmallInput,
            )
        },
    );
}
