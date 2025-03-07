// Based on Duh et al.

use kdtree::KdTree;
use rand::prelude::*;
use rayon::prelude::*;
use std::sync::{Arc, RwLock};

// Two-level spatial search structure
struct SpatialIndex {
    // First level k-d tree (immutable after construction)
    first_level: KdTree<f64, usize, [f64; 2]>,
    // Second level k-d trees (mutable, protected by RwLock)
    second_level: Vec<Arc<RwLock<KdTree<f64, (f64, f64), [f64; 2]>>>>,
}

impl SpatialIndex {
    fn new(seed_points: &[(f64, f64)]) -> Self {
        let mut first_level = KdTree::new(2);
        let mut second_level = Vec::with_capacity(seed_points.len());

        // Build first level with references to seed points
        for (i, point) in seed_points.iter().enumerate() {
            first_level.add([point.0, point.1], i).unwrap();
            second_level.push(Arc::new(RwLock::new(KdTree::new(2))));
        }

        SpatialIndex {
            first_level,
            second_level,
        }
    }

    fn insert_point(&self, point: (f64, f64)) -> bool {
        // Find the closest seed point
        let nearest = self
            .first_level
            .nearest(&[point.0, point.1], 1, &squared_euclidean)
            .unwrap();
        if nearest.is_empty() {
            return false;
        }

        let seed_idx = nearest[0].1;

        // Get write lock on the corresponding second-level tree
        let mut tree = self.second_level[*seed_idx].write().unwrap();

        // Insert the point
        tree.add([point.0, point.1], point).unwrap();
        true
    }

    fn is_valid(&self, point: (f64, f64), min_dist: f64) -> bool {
        // Find nearest seed point
        let nearest_seeds = self
            .first_level
            .nearest(&[point.0, point.1], 3, &squared_euclidean)
            .unwrap();

        // Check nearby second-level trees
        for (_, &seed_idx) in nearest_seeds {
            // Get read lock
            let tree = self.second_level[seed_idx].read().unwrap();

            // Find nearest points in this subtree
            let min_dist_squared = min_dist * min_dist;
            let nearest = tree
                .within(&[point.0, point.1], min_dist_squared, &squared_euclidean)
                .unwrap();

            if !nearest.is_empty() {
                return false; // Found points too close
            }
        }

        true
    }
}

// Distance function for k-d tree
fn squared_euclidean(a: &[f64], b: &[f64]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

struct ParallelPoissonDisk {
    minimum_distance: f64,
    num_samples: usize,
    width: f64,
    height: f64,
    spatial_index: Arc<SpatialIndex>,
    samples: Arc<RwLock<Vec<(f64, f64)>>>,
}

impl ParallelPoissonDisk {
    fn new(w: f64, h: f64, r: f64, k: usize, num_threads: usize) -> Self {
        // Generate seed points with larger radius
        let seed_radius = r * 4.0; // Scaling factor for seed points
        let seed_points = generate_seed_points(w, h, seed_radius, num_threads);

        // Create spatial index with the seed points
        let spatial_index = Arc::new(SpatialIndex::new(&seed_points));

        // Add seed points to samples
        let samples = Arc::new(RwLock::new(seed_points.clone()));

        // Add seed points to the spatial index
        for point in &seed_points {
            spatial_index.insert_point(*point);
        }

        ParallelPoissonDisk {
            minimum_distance: r,
            num_samples: k,
            width: w,
            height: h,
            spatial_index,
            samples,
        }
    }

    fn generate(&self) {
        let seed_points = self.samples.read().unwrap().clone();
        //let num_seeds = seed_points.len();

        // Process seed points in parallel
        seed_points
            .into_par_iter()
            .enumerate()
            .for_each(|(_, seed)| {
                let mut rng = rand::thread_rng();
                let mut active = vec![seed];

                while !active.is_empty() {
                    let idx = (rng.gen::<f64>() * (active.len() - 1) as f64) as usize;
                    let source = active[idx];
                    let mut found = false;

                    for _ in 0..self.num_samples {
                        let new_point = self.generate_around(source);

                        // Check if the point is valid
                        if self.is_valid(new_point) {
                            // Insert into spatial index
                            self.spatial_index.insert_point(new_point);

                            // Add to global samples
                            {
                                let mut samples = self.samples.write().unwrap();
                                samples.push(new_point);
                            }

                            active.push(new_point);
                            found = true;
                        }
                    }

                    if !found {
                        active.remove(idx);
                    }
                }
            });
    }

    fn generate_around(&self, pt: (f64, f64)) -> (f64, f64) {
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

    fn is_valid(&self, point: (f64, f64)) -> bool {
        self.spatial_index.is_valid(point, self.minimum_distance)
    }
}

// Helper function to generate initial seed points
fn generate_seed_points(width: f64, height: f64, min_dist: f64, count: usize) -> Vec<(f64, f64)> {
    // Use the sequential algorithm to generate evenly distributed seed points
    let mut poisson_disk = PoissonDisk::new(width, height, min_dist, 30);
    poisson_disk.generate();

    // If we have fewer points than desired, use what we have
    if poisson_disk.samples.len() <= count {
        return poisson_disk.samples;
    }

    // Otherwise, select evenly distributed subset
    let step = poisson_disk.samples.len() / count;
    poisson_disk
        .samples
        .into_iter()
        .step_by(step)
        .take(count)
        .collect()
}

pub fn generate_points_parallel(
    d: f64,
    width: f64,
    height: f64,
    num_threads: usize,
) -> Vec<(f64, f64)> {
    let threads = num_threads.max(1).min(rayon::current_num_threads());
    println!("Using {} threads", threads);
    let poisson_disk = ParallelPoissonDisk::new(width, height, d, 30, threads);

    poisson_disk.generate();
    let x = poisson_disk.samples.read().unwrap().clone();
    x
}

// Keep the original sequential implementation for generating seed points
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
    generate_points_parallel(d, width, height, 1000)
}
