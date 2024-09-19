use arith::bench_field;
use criterion::{criterion_group, criterion_main, Criterion};
use mersenne31::{M31Ext3, M31Ext3x16, M31x16,  M31};

#[cfg(target_arch = "x86_64")]
use mersenne31::M31x16_256;


fn ext_by_base_benchmark(c: &mut Criterion) {
    bench_field::<M31Ext3>(c);
    bench_field::<M31Ext3x16>(c);
}

fn field_benchmark(c: &mut Criterion) {
    bench_field::<M31>(c);
    bench_field::<M31x16>(c);
    #[cfg(target_arch = "x86_64")]
    bench_field::<M31x16_256>(c);
   
}

criterion_group!(bench, ext_by_base_benchmark, field_benchmark);
criterion_main!(bench);
