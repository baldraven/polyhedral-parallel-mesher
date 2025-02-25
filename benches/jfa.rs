use blue_noise::jfa_wgpu::run;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs::File;
use std::io::{BufRead, BufReader};

fn load_points() -> Vec<(f64, f64)> {
    let file = File::open("benches/mocked_data/points.txt").expect("Failed to open points file");
    let reader = BufReader::new(file);

    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let mut coords = line.split(',');
            let x = coords.next()?.parse::<f64>().ok()?;
            let y = coords.next()?.parse::<f64>().ok()?;
            Some((x, y))
        })
        .collect()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let points = load_points();

    let mut group = c.benchmark_group("jfa");
    group.sample_size(10);
    group.bench_function("jfa_gpu", |b| {
        b.iter(|| run(black_box(&points), black_box((10., 10.))))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
