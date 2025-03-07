pub mod cli;
pub mod plot;

// Re-export commonly used items for convenience
pub use cli::{parse, Cli, JfaMode, Mode, PlotMode};

pub mod generate_cells {
    pub mod jfa {
        pub mod cpu {
            pub mod parallel;
            pub mod sequential;
        }
        pub mod wgpu;
    }
    pub mod mesh_extraction {
        pub mod cpu;
        pub mod wgpu;
    }
}

pub mod generate_points {
    pub mod grid {
        pub mod grid_with_d;
        pub mod grid_with_n;
    }
    pub mod poisson_disk {
        pub mod parallel;
        pub mod sequential;
    }
}

use std::fs::File;
use std::io::Write;

use honeycomb::prelude::CMap2;

pub fn generate_points(cli: &cli::Cli) -> Result<Vec<(f64, f64)>, &'static str> {
    match cli.mode {
        cli::Mode::GridWithN => Ok(generate_points::grid::grid_with_n::generate_points(
            cli.n,
            cli.x as usize,
            cli.y as usize,
        )),
        cli::Mode::GridWithD => Ok(generate_points::grid::grid_with_d::generate_points(
            cli.d, cli.x, cli.y,
        )),
        cli::Mode::PoissonDisk => Ok(generate_points::poisson_disk::sequential::generate_points(
            cli.d, cli.x, cli.y,
        )),
        cli::Mode::PoissonDiskParallel => Ok(
            generate_points::poisson_disk::parallel::generate_points(cli.d, cli.x, cli.y),
        ),
    }
}

pub fn generate_cells(points: &[(f64, f64)], cli: &cli::Cli) -> Result<Vec<usize>, &'static str> {
    match cli.jfa_mode {
        cli::JfaMode::None => Ok(vec![]),
        cli::JfaMode::Gpu => {
            println!("Generating cells using GPU with resolution {}...", cli.reso);
            generate_cells::jfa::wgpu::main(points, (cli.x, cli.y), cli.reso)
        }
        cli::JfaMode::Cpu => {
            println!("Generating cells using CPU with resolution {}...", cli.reso);
            generate_cells::jfa::cpu::sequential::jfa(points, (cli.x, cli.y), cli.reso)
        }
        cli::JfaMode::Rayon => {
            println!(
                "Generating cells using Rayon (parallel CPU) with resolution {}...",
                cli.reso
            );
            generate_cells::jfa::cpu::parallel::jfa(points, (cli.x, cli.y), cli.reso)
        }
    }
}

pub fn generate_mesh(
    pixels: &[usize],
    num_colors: usize,
) -> Result<honeycomb::prelude::CMap2<f32>, &'static str> {
    generate_cells::mesh_extraction::cpu::generate_mesh(pixels, num_colors)
}

pub fn handle_output(
    cli: &cli::Cli,
    points: &Vec<(f64, f64)>,
    pixels: Option<&Vec<usize>>,
    map: Option<&CMap2<f32>>,
) {
    // Export points to a CSV file if specified
    if let Some(ref export_path) = cli.export {
        let mut file = File::create(export_path).expect("Unable to create file");
        for (x, y) in points {
            file.write_all(format!("{},{}\n", x, y).as_bytes())
                .expect("Unable to write data");
        }
        println!("Points written to {}", export_path.display());
    }

    if matches!(cli.plot, cli::PlotMode::Points) {
        println!("Plotting points...");
        plot::plot_points(points);
    }

    if let Some(pixels) = pixels {
        if matches!(cli.plot, cli::PlotMode::Jfa) {
            println!("Plotting cells...");
            plot::plot_heatmap_with_points(pixels, points, (cli.x, cli.y));
        }
    }

    if let Some(map) = map {
        if !cli.no_mesh_visualization {
            println!("Plotting mesh...");
            plot::plot_mesh(map);
        }
    }
}

pub fn load_points(filename: &str) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(filename)?;
    let mut points = Vec::new();

    for line in content.lines() {
        let coords: Vec<f64> = line
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if coords.len() == 2 {
            points.push((coords[0], coords[1]));
        }
    }

    if points.is_empty() {
        return Err("No valid points found in file".into());
    }

    Ok(points)
}
