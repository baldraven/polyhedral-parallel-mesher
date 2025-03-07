fn jfa_step(pixel_grid: &mut [usize], normal_points: &[(usize, usize)], k: usize, reso: usize) {
    for x in 0..reso {
        for y in 0..reso {
            let initial_poisition = x + y * reso;
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
                    let found_color = pixel_grid[new_position];
                    let current_color = pixel_grid[initial_poisition];

                    if (dx == 0 && dy == 0) || found_color == 0 || current_color == found_color {
                        continue;
                    }

                    if current_color == 0 {
                        pixel_grid[initial_poisition] = found_color;
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

                    //dbg!(point1, point2, (x,y), dist1, dist2);

                    if dist2 < dist1 {
                        pixel_grid[initial_poisition] = found_color;
                    }
                }
            }
        }
    }
}

pub fn jfa(
    points: &[(f64, f64)],
    config: (f64, f64),
    reso: u32,
) -> Result<Vec<usize>, &'static str> {
    let reso = reso as usize;
    let normal_points: Vec<(usize, usize)> = points
        .iter()
        .map(|(a, b)| {
            let x = ((a * reso as f64 / config.0).min(reso as f64 - 1.0)) as usize;
            let y = ((b * reso as f64 / config.1).min(reso as f64 - 1.0)) as usize;
            (x, y)
        })
        .collect();

    let mut pixel_grid = vec![0; reso * reso];

    // Mark the initial points on the grid with their respective color
    for (i, point) in normal_points.iter().enumerate() {
        let color = i + 1; // 0 means uncolored
        pixel_grid[point.0 + point.1 * reso] = color;
    }

    // Main JFA loop
    let now = std::time::Instant::now();

    let mut k = (reso / 2).max(1);
    jfa_step(&mut pixel_grid, &normal_points, 1, reso); // 1+JFA for more precision
    while k >= 1 {
        //println!("Entering loop with k = {}", k);
        jfa_step(&mut pixel_grid, &normal_points, k, reso);
        k /= 2;
    }

    let elapsed = now.elapsed();
    println!("{:.2?}", elapsed);

    Ok(pixel_grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_pixel() {
        let points = vec![(1.0, 1.0)];
        let config = (2.0, 2.0);
        let reso = 512;

        let pixel_grid = jfa(&points, config, reso).unwrap();

        // Check middle of the grid
        assert_eq!(
            pixel_grid[(reso as usize) * (reso as usize) / 2 + (reso as usize) / 2],
            1
        );

        // For the previously hardcoded values (approximate equivalent)
        let expected_index =
            (reso as f64 * 1.0 / 2.0) as usize + (reso as f64 * 1.0 / 2.0) as usize * reso as usize;
        assert_eq!(pixel_grid[expected_index], 1);
    }
}
