use honeycomb::prelude::CMap2;
use honeycomb::render::App;
use plotly::common::{ColorScalePalette, Mode};
use plotly::{HeatMap, Layout, Plot, Scatter};

use rand::seq::SliceRandom; // Requires the `rand` crate
use rand::thread_rng;

pub fn plot_heatmap_with_points(
    data: &[usize],
    points: &[(f64, f64)],
    config_dimension: (f64, f64),
) {
    let reso = (data.len() as f64).sqrt() as usize;

    // Reshape the data into a 2D grid (Vec<Vec<usize>>)
    let mut grid: Vec<Vec<usize>> = vec![vec![0; reso]; reso];
    for i in 0..reso {
        for j in 0..reso {
            grid[i][j] = data[i * reso + j];
        }
    }

    // Generate the range of colors and shuffle them
    let mut colors: Vec<usize> = (1..=points.len()).collect();
    /*     colors.shuffle(&mut thread_rng());
     */
    // Create a mapping from region value to shuffled color index
    let value_to_color: Vec<usize> = colors;

    // Map the grid to the new color indices
    let grid_mapped: Vec<Vec<f64>> = grid
        .iter()
        .map(|row| {
            row.iter()
                .map(|&value| value_to_color[value - 1] as f64) // Map based on the shuffled color
                .collect()
        })
        .collect();

    let colorscale = ColorScalePalette::Viridis;
    let heatmap = HeatMap::new_z(grid_mapped).color_scale(colorscale.into());

    let normalized_points: Vec<(f64, f64)> = points
        .iter()
        .map(|(px, py)| {
            (
                *px * reso as f64 / config_dimension.0,
                *py * reso as f64 / config_dimension.1,
            )
        }) // Scale points to match the heatmap resolution
        .collect();

    // Prepare the points for scatter plot
    let x_list: Vec<f64> = normalized_points.iter().map(|(x, _)| *x).collect();
    let y_list: Vec<f64> = normalized_points.iter().map(|(_, y)| *y).collect();
    let scatter = Scatter::new(x_list, y_list)
        .mode(Mode::Markers)
        .marker(plotly::common::Marker::new().size(40).color("#000000")); // Black markers for points

    let mut plot = Plot::new();
    plot.add_trace(heatmap);
    plot.add_trace(scatter);

    let layout = Layout::new().height(2048).width(2048).auto_size(false);
    plot.set_layout(layout);

    plot.show();
}

pub fn plot_points(points: &[(f64, f64)]) {
    let x_list = points.iter().map(|(x, _)| *x).collect();
    let y_list = points.iter().map(|(_, y)| *y).collect();
    let mut plot = Plot::new();

    let trace = Scatter::new(x_list, y_list)
        .mode(Mode::Markers)
        .marker(plotly::common::Marker::new().size(40)); // Black markers for points
    plot.add_trace(trace);
    let layout = Layout::new().height(2048).width(2048).auto_size(false);

    plot.set_layout(layout);
    plot.show();
}

pub fn plot_mesh(map: &CMap2<f32>) {
    let mut render_app = App::default();
    render_app.add_capture(&map);
    render_app.run();
}
