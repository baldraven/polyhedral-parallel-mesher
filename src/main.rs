use blue_noise::*;

fn main() {
    let cli = cli::parse();
    cli::print_config(&cli);

    // Processing
    let points = generate_points(&cli).unwrap_or_else(|err| {
        println!("Problem generating points: {err}");
        std::process::exit(1);
    });

    let pixels = generate_cells(&points, &cli).unwrap_or_else(|err| {
        println!("Problem running JFA: {err}");
        std::process::exit(1);
    });

    
    let mesh = generate_mesh(&pixels, points.len()).unwrap_or_else(|err| {
        println!("Problem generating mesh: {err}");
        std::process::exit(1);
    });

    // Visualize
    handle_output(&cli, &points, Some(&pixels), Some(&mesh));
}
