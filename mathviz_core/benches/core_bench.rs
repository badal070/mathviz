use criterion::{criterion_group, criterion_main, Criterion};

fn bench_surface_512(c: &mut Criterion) {
    c.bench_function("bench_surface_512", |b| b.iter(|| ()));
}

criterion_group!(benches, bench_surface_512);
criterion_main!(benches);
