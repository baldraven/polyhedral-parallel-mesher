use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Point generation on a rectangle.
#[derive(Parser, Debug)]
#[command(version, about = "Point generation on a rectangle.")]
pub struct Cli {
    /// Sets the point generation mode
    #[arg(
        short = 'g',
        long = "generation",
        default_value = "poisson-disk-parallel",
        value_enum
    )]
    pub mode: Mode,

    /// Sets the number of points to generate. Only works with `grid-with-n` mode
    #[arg(short = 'n', default_value_t = 10)]
    pub n: u32,

    /// Sets the minimal distance of points
    #[arg(short = 'd', long = "distance", default_value_t = 1.0)]
    pub d: f64,

    /// Sets the width of the box
    #[arg(short = 'x', default_value_t = 10.0)]
    pub x: f64,

    /// Sets the height of the box
    #[arg(short = 'y', default_value_t = 10.0)]
    pub y: f64,

    /// Exports point list to a CSV-formatted file
    #[arg(short = 'e', long = "export", value_name = "FILE")]
    pub export: Option<PathBuf>,

    /// Plot options
    #[arg(short = 'p', long = "plot", default_value = "jfa", value_enum)]
    pub plot: PlotMode,

    /// Sets the JFA mode
    #[arg(short = 'j', long = "jfa-mode", default_value = "gpu", value_enum)]
    pub jfa_mode: JfaMode,

    /// Sets the resolution for JFA
    #[arg(short = 'r', long = "reso", default_value_t = 1024)]
    pub reso: u32,

    // Disable honeycomb visualization
    #[arg(long = "no-mesh-visualization")]
    pub no_mesh_visualization: bool,
    
    /// Sets the number of threads for parallel processing (Rayon)
    #[arg(long = "threads", default_value_t = 0)]
    pub num_threads: usize,
}

/// Point generation modes
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
pub enum Mode {
    GridWithN,
    GridWithD,
    PoissonDisk,
    PoissonDiskParallel,
}

/// Plotting options
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
pub enum PlotMode {
    Points,
    Jfa,
    None,
}

/// JFA modes
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
pub enum JfaMode {
    Cpu,
    Gpu,
    Rayon,
    None,
}

pub fn print_config(cli: &Cli) {
    println!("Display help with option -h or --help.");
    println!("The program will run with the following configuration:");
    println!("Mode: {:?}", cli.mode);
    println!("Number of points (n): {}", cli.n);
    println!("Minimal distance (d): {}", cli.d);
    println!(
        "Box dimensions: width (x) = {}, height (y) = {}",
        cli.x, cli.y
    );
    if let Some(ref export_path) = cli.export {
        println!("Export path: {}", export_path.display());
    }
    println!("Plot mode: {:?}", cli.plot);
    println!("JFA mode: {:?}", cli.jfa_mode);
    if cli.jfa_mode != JfaMode::None {
        println!("JFA resolution: {}", cli.reso);
        if cli.jfa_mode == JfaMode::Rayon && cli.num_threads > 0 {
            println!("Number of threads: {}", cli.num_threads);
        }
    }
    if cli.no_mesh_visualization {
        println!("Mesh visualization disabled");
    }
    println!();
}

pub fn parse() -> Cli {
    Cli::parse()
}
