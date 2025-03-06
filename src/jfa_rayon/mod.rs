use rayon::prelude::*;
fn jfa_step_parallel(
    pixel_grid: &mut [usize], 
    normal_points: &[(usize, usize)], 
    k: usize, 
    reso: usize
) {
    // Create a copy of the grid for reading to avoid race conditions
    let pixel_grid_read = pixel_grid.to_vec();
    
    // Process all pixels in parallel using indexed mutable iteration
    pixel_grid.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
        // Calculate x and y from 1D index
        let x = idx % reso;
        let y = idx / reso;
        
        let current_color = pixel_grid_read[idx];
        let mut closest_color = current_color;
        let mut min_distance = f64::MAX;
        
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

                if (dx == 0 && dy == 0) || found_color == 0 || current_color == found_color {
                    continue;
                }

                // If the current pixel has no color, we can simply adopt the found color
                if current_color == 0 {
                    closest_color = found_color;
                    break;
                }

                // Compare distances to determine the closer point
                let point1 = normal_points[current_color - 1];
                let point2 = normal_points[found_color - 1];

                let dist2 = ((x as isize - point2.0 as isize).pow(2)
                    + (y as isize - point2.1 as isize).pow(2))
                    as f64;
                
                // Only calculate distance for current color if needed
                if dist2 < min_distance {
                    let dist1 = ((x as isize - point1.0 as isize).pow(2)
                        + (y as isize - point1.1 as isize).pow(2))
                        as f64;
                    
                    if dist2 < dist1 {
                        closest_color = found_color;
                        min_distance = dist2;
                    }
                }
            }
        }
        
        // Update the pixel if we found a closer color
        if closest_color != current_color {
            *pixel = closest_color;  // Direct assignment to the mutable reference
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
