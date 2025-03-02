use rayon::prelude::*;

// Parallel step function that processes a chunk of rows using rayon
fn jfa_step_parallel(
    pixel_grid: &mut [usize], 
    normal_points: &[(usize, usize)], 
    k: usize, 
    reso: usize
) {
    // Create a copy of the grid for reading to avoid race conditions
    let pixel_grid_read = pixel_grid.to_vec();
    
    // Process rows in parallel using chunks to modify the grid safely
    pixel_grid.chunks_mut(reso).enumerate().par_bridge().for_each(|(y, row)| {
        for x in 0..reso {
            let initial_position = x;
            // Check the 8-neighborhood (jump in all directions) and update to the closest point
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    let new_x = x as isize + dx * k as isize;
                    let new_y = y as isize + dy * k as isize;

                    if !(new_x >= 0 && new_x < reso as isize && new_y >= 0 && new_y < reso as isize)
                    {
                        continue;
                    }

                    let new_position = (new_x as usize) + (new_y as usize) * reso;
                    let found_color = pixel_grid_read[new_position];
                    let current_color = pixel_grid_read[x + y * reso];

                    if (dx == 0 && dy == 0) || found_color == 0 || current_color == found_color {
                        continue;
                    }

                    if current_color == 0 {
                        row[initial_position] = found_color;
                        continue;
                    }

                    // we're now in the case where we have two colors distinct colors
                    // so we'll assign the closest color to the current pixel
                    let point1 = normal_points[current_color - 1];
                    let point2 = normal_points[found_color - 1];

                    let dist1 = ((x as isize - point1.0 as isize).pow(2)
                        + (y as isize - point1.1 as isize).pow(2))
                        as f64;
                    let dist2 = ((x as isize - point2.0 as isize).pow(2)
                        + (y as isize - point2.1 as isize).pow(2))
                        as f64;

                    if dist2 < dist1 {
                        row[initial_position] = found_color;
                    }
                }
            }
        }
    });
}

pub fn jfa(points: &[(f64, f64)], config: (f64, f64), reso: u32) -> Result<Vec<usize>, &'static str> {
    let reso = reso as usize;
    let normal_points: Vec<(usize, usize)> = points
        .iter()
        .map(|(a, b)| {
            let x = ((a * reso as f64 / config.0).min(reso as f64 - 1.0)) as usize;
            let y = ((b * reso as f64 / config.1).min(reso as f64 - 1.0)) as usize;
            (x, y)
        })
        .collect();

    let pixel_grid = vec![0; reso * reso];

    // Mark the initial points on the grid with their respective color
    // Using a thread-safe approach with a mutex
    use std::sync::Mutex;
    let pixel_grid = Mutex::new(pixel_grid);
    
    normal_points.par_iter().enumerate().for_each(|(i, point)| {
        let color = i + 1; // 0 means uncolored
        let index = point.0 + point.1 * reso;
        // Use mutex to safely update the grid
        let mut grid = pixel_grid.lock().unwrap();
        grid[index] = color;
    });
    
    // Unwrap the mutex to get the grid back
    let mut pixel_grid = pixel_grid.into_inner().unwrap();

    // Main JFA loop

    let mut k = (reso / 2).max(1);
    jfa_step_parallel(&mut pixel_grid, &normal_points, 1, reso); // 1+JFA for more precision
    while k >= 1 {
        jfa_step_parallel(&mut pixel_grid, &normal_points, k, reso);
        k /= 2;
    }

    Ok(pixel_grid)
}
