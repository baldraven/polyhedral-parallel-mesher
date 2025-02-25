use std::time::Instant;
use std::io::Write;

use blue_noise::*;


fn main() {
    let cli = cli::parse();
    cli::print_config(&cli);

    let start = Instant::now();

/*     // Processing
    let points = generate_points(&cli).unwrap_or_else(|err| {
        println!("Problem generating points: {err}");
        std::process::exit(1);
    }); */

/*     // write points to file
    let mut file = std::fs::File::create("points.txt").unwrap_or_else(|err| {
        println!("Problem creating file: {err}");
        std::process::exit(1);
    });
    for point in &points {
        writeln!(file, "{} {}", point.0, point.1).unwrap_or_else(|err| {
            println!("Problem writing to file: {err}");
            std::process::exit(1);
        });
    }
 */

    // We are in MOCKMODE. so we'll load the points instead of randomly generating them
    let points = load_points("points.txt").unwrap_or_else(|err| {
        println!("Problem loading points: {err}");
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

    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);

    // Visualize
    handle_output(&cli, &points, Some(&pixels), Some(&mesh));
}
