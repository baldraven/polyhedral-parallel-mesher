fn best_grid_dimensions(n: usize, x: usize, y: usize) -> (usize, usize) {
    let target_aspect_ratio = x as f64 / y as f64;
    let mut best_r = 1;
    let mut best_c = n;
    let mut best_ratio_diff = f64::MAX;

    for r in 1..=n {
        if n % r == 0 {
            let c = n / r;

            // Calculate the aspect ratio for the current grid configuration
            let current_aspect_ratio = c as f64 / r as f64;

            // Calculate the difference from the target aspect ratio
            let ratio_diff = (current_aspect_ratio - target_aspect_ratio).abs();

            // Check if this configuration is closer to the target aspect ratio
            if ratio_diff < best_ratio_diff {
                best_ratio_diff = ratio_diff;
                best_r = r;
                best_c = c;
            }
        }
    }

    (best_c, best_r)
}

pub fn generate_points(n: u32, width: usize, height: usize) -> Vec<(f64, f64)> {
    let mut points = Vec::new();

    let (cols, rows) = best_grid_dimensions(n as usize, width, height);

    // Calculate the spacing between points
    let x_spacing = width as f64 / (cols + 1) as f64;
    let y_spacing = height as f64 / (rows + 1) as f64;

    for r in 0..rows {
        for c in 0..cols {
            // Calculate the coordinates of each point
            let x = (c + 1) as f64 * x_spacing;
            let y = (r + 1) as f64 * y_spacing;
            points.push((x, y));
        }
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_dimension() {
        let (r, c) = best_grid_dimensions(16, 8, 8);
        assert_eq!(r, 4);
        assert_eq!(c, 4);

        let (r, c) = best_grid_dimensions(16, 1, 44);
        assert_eq!(r, 1);
        assert_eq!(c, 16);
    }

    #[test]
    fn test_fit_points_in_rectangle() {
        let n = 12;
        let width = 10;
        let height = 5;

        let expected_points = [
            (2., 1.25),
            (4., 1.25),
            (6., 1.25),
            (8., 1.25),
            (2., 2.5),
            (4., 2.5),
            (6., 2.5),
            (8., 2.5),
            (2., 3.75),
            (4., 3.75),
            (6., 3.75),
            (8., 3.75),
        ];

        let points = generate_points(n, width, height);

        assert_eq!(points.len(), expected_points.len());

        for (point, expected) in points.iter().zip(expected_points.iter()) {
            assert!((point.0 - expected.0).abs() < f64::EPSILON);
            assert!((point.1 - expected.1).abs() < f64::EPSILON);
        }
    }
}
