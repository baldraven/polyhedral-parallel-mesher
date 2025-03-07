// https://a5huynh.github.io/posts/2019/poisson-disk-sampling/
// Based on Bridson

use rand::prelude::*;
struct PoissonDisk {
    minimum_distance: f64,
    num_samples: usize,
    width: f64,
    height: f64,
    cell_size: f64,
    grid_width: f64,
    grid_height: f64,
    grid: Vec<Option<(f64, f64)>>,
    samples: Vec<(f64, f64)>,
    active: Vec<(f64, f64)>,
}

impl PoissonDisk {
    fn new(w: f64, h: f64, r: f64, k: usize) -> Self {
        let cell_size = r / 2.0_f64.sqrt();
        let grid_width = (w / cell_size).ceil() + 1.0;
        let grid_height = (h / cell_size).ceil() + 1.0;

        let mut disk = PoissonDisk {
            minimum_distance: r,
            num_samples: k,
            width: w,
            height: h,
            cell_size,
            grid_width,
            grid_height,
            grid: vec![None; (grid_width * grid_height) as usize],
            samples: Vec::new(),
            active: Vec::new(),
        };

        let mut rng = rand::thread_rng();
        let point = ((rng.gen::<f64>() * w), (rng.gen::<f64>() * h));

        // Add point to grid & active list.
        disk.insert_point(point);
        disk.active.push(point);

        disk
    }

    fn insert_point(&mut self, point: (f64, f64)) {
        // Calculate the (x, y) coordinate when place inside the grid.
        let cell_x = (point.0 / self.cell_size).floor();
        let cell_y = (point.1 / self.cell_size).floor();

        // Calculate the index within our flat array and place the point there.
        let cell_idx = (cell_y * self.grid_width + cell_x) as usize;
        self.grid[cell_idx] = Some(point);
    }

    fn generate_around(&mut self, pt: (f64, f64)) -> (f64, f64) {
        // Random angle and radius between r and 2r
        let mut rng = rand::thread_rng();
        let angle = 2.0 * std::f64::consts::PI * rng.gen::<f64>();
        let radius = self.minimum_distance * (rng.gen::<f64>() + 1.0);

        let new_x = pt.0 + (radius * angle.cos());
        let new_y = pt.1 + (radius * angle.sin());

        (
            new_x.max(0.0).min(self.width - 1.0),
            new_y.max(0.0).min(self.height - 1.0),
        )
    }

    fn distance(&self, pa: (f64, f64), pb: (f64, f64)) -> f64 {
        let dx = pa.0 - pb.0;
        let dy = pa.1 - pb.1;
        (dx * dx + dy * dy).sqrt()
    }

    fn is_valid(&self, point: (f64, f64)) -> bool {
        let xidx = (point.0 / self.cell_size).floor();
        let yidx = (point.1 / self.cell_size).floor();

        // Determine the neighborhood around the source point.
        let start_x = (xidx - 2.0).max(0.0) as usize;
        let end_x = (xidx + 2.0).min(self.grid_width - 1.0) as usize;
        let start_y = (yidx - 2.0).max(0.0) as usize;
        let end_y = (yidx + 2.0).min(self.grid_height - 1.0) as usize;

        // Check all non-empty neighbors cells and make sure the new point is outside their radius.
        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = y * self.grid_width as usize + x;
                if let Some(cell) = self.grid[idx] {
                    if self.distance(cell, point) <= self.minimum_distance {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn generate(&mut self) {
        let mut rng = rand::thread_rng();
        while !self.active.is_empty() {
            let idx = (rng.gen::<f64>() * (self.active.len() - 1) as f64) as usize;
            let source = self.active[idx];
            let mut found = false;

            for _ in 0..self.num_samples {
                let new_point = self.generate_around(source);

                if self.is_valid(new_point) {
                    self.insert_point(new_point);
                    self.active.push(new_point);
                    self.samples.push(new_point);
                    found = true;
                }
            }
            if !found {
                self.active.remove(idx);
            }
        }
    }
}

pub fn generate_points(d: f64, width: f64, height: f64) -> Vec<(f64, f64)> {
    let mut poisson_disk = PoissonDisk::new(width, height, d, 30);
    poisson_disk.generate();
    poisson_disk.samples
}
