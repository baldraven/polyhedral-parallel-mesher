use blue_noise::generate_mesh;
use blue_noise::mesh::extract_voronoi_cell_vertices;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn read_benchmark_data() -> (Vec<usize>, usize) {
    let file = File::open("benches/mocked_data/benchmark_data_d01_16jan.txt").expect("Failed to open benchmark data");
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    
    // Read first line for pixels
    let pixels_line = lines.next().expect("Missing pixels line").expect("Failed to read pixels line");
    let pixels: Vec<usize> = pixels_line
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|x| x.trim().parse().expect("Failed to parse pixel"))
        .collect();
    
    // Read second line for num_color
    let num_color_line = lines.next().expect("Missing num_color line").expect("Failed to read num_color line");
    let num_color: usize = num_color_line.trim().parse().expect("Failed to parse num_color");
    
    (pixels, num_color)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let (pixels, num_color) = read_benchmark_data();

    let mut group = c.benchmark_group("generate_mesh");
    group.sample_size(30);
    group.bench_function("generate_mesh", |b| b.iter(|| generate_mesh(black_box(&pixels), black_box(num_color))));


    let res = (pixels.len() as f64).sqrt() as usize;
    let mut color_vertices: Vec<Vec<Vec<usize>>> = vec![Vec::new(); num_color];
    let mut vertex_map: HashMap<Vec<usize>, (u32, u32)> = HashMap::new();

    group.bench_function("extract_voronoi_cell_vertices", |b| {
        b.iter(|| {
            let _ = extract_voronoi_cell_vertices(black_box(&pixels), black_box(res), black_box(&mut color_vertices), black_box(&mut vertex_map));
        })
    });

    group.finish();
}
    

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

