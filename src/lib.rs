pub mod cli;
pub mod jfa_cpu;
pub mod jfa_wgpu;
pub mod mesh;
pub mod mesh_wgpu;
mod mode1;
mod mode2;
pub mod mode3;
mod plot;

use std::fs::File;
use std::io::Write;

use honeycomb::prelude::CMap2;

pub fn generate_points(cli: &cli::Cli) -> Result<Vec<(f64, f64)>, &'static str> {
    match cli.mode {
        cli::Mode::GridWithN => Ok(mode1::generate_points(
            cli.n,
            cli.x as usize,
            cli.y as usize,
        )),
        cli::Mode::GridWithD => Ok(mode2::generate_points(cli.d, cli.x, cli.y)),
        cli::Mode::PoissonDisk => Ok(mode3::generate_points(cli.d, cli.x, cli.y)),
    }
}

pub fn generate_cells(points: &[(f64, f64)], cli: &cli::Cli) -> Result<Vec<usize>, &'static str> {
    match cli.jfa_mode {
        cli::JfaMode::None => Ok(vec![]),
        cli::JfaMode::Gpu => {
            println!("Generating cells using GPU with resolution {}...", cli.res);
            jfa_wgpu::main(points, (cli.x, cli.y))
        }
        cli::JfaMode::Cpu => {
            println!("Generating cells using CPU with resolution {}...", cli.res);
            jfa_cpu::jfa(points, (cli.x, cli.y))
        }
    }
}

pub fn generate_mesh(pixels: &[usize], num_colors: usize) -> Result<honeycomb::prelude::CMap2<f32>, &'static str> {
    mesh::generate_mesh(pixels, num_colors)
}

pub fn handle_output(cli: &cli::Cli, points: &Vec<(f64, f64)>, pixels: Option<&Vec<usize>>, map: Option<&CMap2<f32>>) {
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
