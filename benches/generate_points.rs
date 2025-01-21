use blue_noise::mode3::generate_points;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("generate_points");
    group.sample_size(100);
    group.bench_function("generate_points", |b| b.iter(|| generate_points(black_box(0.1), black_box(10.), black_box(10.))));
    group.finish();
}
    

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

