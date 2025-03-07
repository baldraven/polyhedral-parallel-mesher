pub fn generate_points(d: f64, width: f64, height: f64) -> Vec<(f64, f64)> {
    let mut points = Vec::new();

    // Calculate how many points fit along the width and height based on distance `d`
    let num_points_x = (width / d).floor() as usize;
    let num_points_y = (height / d).floor() as usize;

    // Generate the points in a grid pattern
    for i in 0..=num_points_x {
        for j in 0..=num_points_y {
            let x = i as f64 * d;
            let y = j as f64 * d;
            points.push((x, y));
        }
    }

    points
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_grid_points_count() {
        // Test with a 100x100 rectangle and minimum distance of 10
        let points = generate_points(10.0, 100.0, 100.0);

        // Expecting a 11x11 grid (since 100 / 10 = 10, but we include 0)
        let expected_num_points = (10 + 1) * (10 + 1); // 11 * 11 = 121
        assert_eq!(points.len(), expected_num_points);
    }

    #[test]
    fn test_generate_grid_points_spacing() {
        // Test with a 30x30 rectangle and minimum distance of 10
        let points = generate_points(10.0, 30.0, 30.0);

        // Ensure that points are spaced by at least `d = 10.0` along the grid
        for (i, &(x1, y1)) in points.iter().enumerate() {
            for (j, &(x2, y2)) in points.iter().enumerate() {
                if i != j {
                    let distance_x = (x2 - x1).abs();
                    let distance_y = (y2 - y1).abs();

                    // Points should be at least 10 units apart on the grid
                    assert!(
                        distance_x >= 10.0 || distance_y >= 10.0,
                        "Points ({}, {}) and ({}, {}) are too close",
                        x1,
                        y1,
                        x2,
                        y2
                    );
                }
            }
        }
    }

    #[test]
    fn test_no_points_generated() {
        // Test where the distance is larger than the rectangle's dimensions
        let points = generate_points(100.0, 50.0, 50.0);

        // Should return only the (0,0) point
        assert_eq!(points.len(), 1);
        assert_eq!(points[0], (0.0, 0.0));
    }
}
