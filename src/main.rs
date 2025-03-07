use blue_noise::*;
use std::time::Instant;

fn main() {
    let cli = cli::parse();
    cli::print_config(&cli);

    let start_points = Instant::now();
    let points = generate_points(&cli).unwrap_or_else(|err| {
        println!("Problem generating points: {err}");
        std::process::exit(1);
    });
    let duration_points = start_points.elapsed();
    println!("Time elapsed for point generation: {:?}", duration_points);

    let start_jfa = Instant::now();
    let pixels = generate_cells(&points, &cli).unwrap_or_else(|err| {
        println!("Problem running JFA: {err}");
        std::process::exit(1);
    });
    let duration_jfa = start_jfa.elapsed();
    println!("Time elapsed for JFA: {:?}", duration_jfa);

    let start_mesh = Instant::now();
    let mesh = generate_mesh(&pixels, points.len()).unwrap_or_else(|err| {
        println!("Problem generating mesh: {err}");
        std::process::exit(1);
    });
    let duration_mesh = start_mesh.elapsed();
    println!("Time elapsed for mesh generation: {:?}", duration_mesh);

    // Visualize
    handle_output(&cli, &points, Some(&pixels), Some(&mesh));
}
