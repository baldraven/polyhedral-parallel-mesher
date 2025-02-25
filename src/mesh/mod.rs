use honeycomb::core::cmap::CMap2;
use honeycomb::prelude::{CMapBuilder, DartIdType, Orbit2, OrbitPolicy, Vertex2};
use plotly::color;
use wgpu::hal::auxil::db;
use std::collections::HashMap;
use std::io::Write;
use std::process::exit;
use std::time::Instant; // Add this import

fn is_subset(sub: &[usize], sup: &[usize]) -> bool {
    sub.iter().all(|x| sup.contains(x))
}

// If some vertices are a subset of others, keep the one with the most colors
// e.g., input [[[1, 2, 3], [1, 2, 3, 4]], [1,2,3]] outputs [[[1, 2, 3, 4]],[[1, 2, 3, 4]]]
// input vec![vec![vec![1, 2, 4], vec![1, 2, 5]], vec![vec![1, 2, 6], vec![1, 2, 7]]] outputs vec![vec![vec![1, 2, 4], vec![1, 2, 5]], vec![vec![1, 2, 6], vec![1, 2, 7]]]
// input vec![vec![vec![1, 2, 3], vec![1, 2, 3, 4]], vec![vec![1, 2, 5], vec![1, 2, 4, 5]]] outputs vec![vec![vec![1, 2, 3, 4]], vec![vec![1, 2, 4, 5]]]
fn remove_subsets_quadratic(
    vertices: &mut Vec<Vec<Vec<usize>>>,
    vertex_map: &mut HashMap<Vec<usize>, (u32, u32)>,
) {
    let mut max_vertices: HashMap<Vec<usize>, Vec<usize>> = HashMap::new();

    // First pass: find all subsets and their corresponding maximal sets
    for vertex_list in vertices.iter() {
        for i in 0..vertex_list.len() {
            for j in 0..vertex_list.len() {
                if i != j && is_subset(&vertex_list[i], &vertex_list[j]) {
                    let current_max = max_vertices
                        .entry(vertex_list[i].clone())
                        .or_insert_with(|| vertex_list[i].clone());
                    if vertex_list[j].len() > current_max.len() {
                        *current_max = vertex_list[j].clone();
                    }
                    // Remove the subset from vertex_map since it will be replaced
                    vertex_map.remove(&vertex_list[i]);
                }
            }
        }
    }

    // Second pass: replace all subsets with their maximal sets
    for vertex_list in vertices.iter_mut() {
        let mut has_subset = false;
        let mut max_vertex = vertex_list[0].clone();

        for v in vertex_list.iter() {
            if let Some(max) = max_vertices.get(v) {
                has_subset = true;
                if max.len() > max_vertex.len() {
                    max_vertex = max.clone();
                }
                // Remove the subset from vertex_map
                vertex_map.remove(v);
            }
        }

        if has_subset {
            *vertex_list = vec![max_vertex];
        }
    }
}

fn get_unique_colors(x: u32, y: u32, res: usize, grid: &[usize]) -> Vec<usize> {
    [
        (x.saturating_sub(1), y.saturating_sub(1)),
        (x, y.saturating_sub(1)),
        (x + 1, y.saturating_sub(1)),
        (x.saturating_sub(1), y),
        (x + 1, y),
        (x.saturating_sub(1), y + 1),
        (x, y + 1),
        (x + 1, y + 1),
    ]
    .iter()
    .map(|&(nx, ny)| {
        if nx > 0 && ny > 0 && nx < res as u32 && ny < res as u32 {
            grid[ny as usize * res + nx as usize]
        } else {
            0
        }
    })
    .collect()
}

/// Extracts the vertices of the Voronoi cells from the pixel grid.
/// A vertex is only valid if there are 3 or more unique colors (0 for boundary).
/// The keys of `Vertex_map` are the ordered Vec<usize> of the adjacent colors of the vertices, and are used in `color_vertices`, that tracks the vertices of each Voronoi cell.
pub fn extract_voronoi_cell_vertices(
    grid: &[usize],
    res: usize,
    color_vertices: &mut [Vec<Vec<usize>>],
    vertex_map: &mut HashMap<Vec<usize>, (u32, u32)>,
) {
    let start = Instant::now();

    for (idx, &current_color) in grid.iter().enumerate() {
        let x = (idx % res) as u32;
        let y = (idx / res) as u32;

        // Collect unique colors in the 8-neighborhood
        let mut unique_colors = get_unique_colors(x, y, res, grid);

        unique_colors.sort();
        unique_colors.dedup();

        if unique_colors.len() >= 3 {
            vertex_map.entry(unique_colors.clone()).or_insert((x, y));

            if current_color > 0 {
                // 0 is the boundary color
                let color_vertice = &mut color_vertices[current_color - 1];
                if !color_vertice.contains(&unique_colors) {
                    color_vertice.push(unique_colors);
                }
            }
        }
    }

    let duration = start.elapsed();
    println!(
        "Time elapsed in extract_voronoi_cell_vertices: {:?}",
        duration
    );
}

/// Returns the number of common elements between two sorted vectors
fn count_common_elements(a: &[usize], b: &[usize]) -> usize {
    let mut count = 0;
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                count += 1;
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    count
}

fn choose_next_vertex(
    current: &Vec<usize>,
    candidates: Vec<(usize, &Vec<usize>)>,
    vertex_map: &HashMap<Vec<usize>, (u32, u32)>,
    sorted: &[Vec<usize>],
) -> usize {
    let pos_current = vertex_map.get(current).unwrap();
    let pos_prev = if sorted.len() > 1 {
        vertex_map.get(&sorted[sorted.len() - 2]).unwrap()
    } else {
        vertex_map.get(candidates[1].1).unwrap()
    };

    candidates
        .into_iter()
        .max_by_key(|(_, v)| {
            let pos_candidate = vertex_map.get(*v).unwrap();
            let dx1 = pos_current.0 as i32 - pos_prev.0 as i32;
            let dy1 = pos_current.1 as i32 - pos_prev.1 as i32;
            let dx2 = pos_candidate.0 as i32 - pos_current.0 as i32;
            let dy2 = pos_candidate.1 as i32 - pos_current.1 as i32;
            dx1 * dy2 - dy1 * dx2
        })
        .unwrap()
        .0
}

/// Sort vertices of a face using topological information
pub fn sort_vertices_topologically(
    vertices: &mut Vec<Vec<usize>>,
    vertex_map: &HashMap<Vec<usize>, (u32, u32)>,
) -> bool {
    assert!(vertices.len() >= 3);

/*     dbg!(&vertices);
    dbg!(&vertex_map);
    print!("__________________________"); */

    let mut sorted = Vec::with_capacity(vertices.len());
    let mut used = vec![false; vertices.len()];
    // Start with first vertex
    sorted.push(vertices[0].clone());
    used[0] = true;

    // For each position to fill
    while sorted.len() < vertices.len() {
        let current = sorted.last().unwrap();

        // Find next vertex (should share exactly 2 colors with current)
        let mut candidates = Vec::new();
        for (i, v) in vertices.iter().enumerate() {
            if !used[i] && count_common_elements(current, v) == 2 {
                candidates.push((i, v));
            }
        }

        match candidates.len() {
            0 => {
                // TODO: investigate in what cases this can happen
                //dbg!("reached here");
                dbg!(sorted.len());
                dbg!(vertices.len());
                dbg!(vertices);
                dbg!(&sorted);
                dbg!(used);
                dbg!(current);
                println!("Warning: incoherent face vertices");
                return false
            }
            1 => {
                // last iteration
                let (idx, v) = candidates[0];
                sorted.push(v.clone());
                used[idx] = true;
            }
            _ => {
                // If we have 2 candidates (should happen only for the first vertex),
                // choose the one that makes a counterclockwise turn
                let chosen_idx = choose_next_vertex(current, candidates, vertex_map, &sorted);
                sorted.push(vertices[chosen_idx].clone());
                used[chosen_idx] = true;
            }
        }
    }

    assert!(count_common_elements(sorted.first().unwrap(), sorted.last().unwrap()) >= 2);
    *vertices = sorted;
    return true
}

/// Generates a combinatorial map from a pixel grid by sewing darts between vertices.
/// Each face in the map corresponds to a color region in the pixel grid.
pub fn generate_mesh(pixels: &[usize], num_colors: usize) -> Result<CMap2<f32>, &'static str> {
    let start = Instant::now();

    let res = (pixels.len() as f64).sqrt() as usize;
    let mut color_vertices: Vec<Vec<Vec<usize>>> = vec![Vec::new(); num_colors];
    let mut vertex_map: HashMap<Vec<usize>, (u32, u32)> = HashMap::new();
    let mut vertices_id: HashMap<Vec<usize>, u32> = HashMap::new();
    let mut edges: HashMap<(&[usize], &[usize]), u32> = HashMap::new();

    extract_voronoi_cell_vertices(pixels, res, &mut color_vertices, &mut vertex_map);



    //write vertex_map into a file
    let mut file = std::fs::File::create("vertex_map.txt").unwrap();
    for (key, value) in &vertex_map {
        writeln!(file, "{:?} {:?}", key, value).unwrap();
    }

    let mut file = std::fs::File::create("color.txt").unwrap();
    for (i, vertices) in color_vertices.iter().enumerate() {
        writeln!(file, "{:?} {:?}", i, vertices).unwrap();
    }




    let mut map: CMap2<f32> = CMapBuilder::default().build().unwrap();

    let mut dart_id = 1;

/* 
remove_subsets_quadratic(&mut color_vertices, &mut vertex_map); // We might want to change the logic here if we see significant benefits thanks to profiling, having higher resolution could work
 */ // THIS FUNCTION DOES NOTHING IN THIS CASE



    let face_vertices_mock = [
        [
            0,
            16,
            34,
        ],
        [
            10,
            16,
            34,
        ],
        [
            10,
            16,
            19,
        ],
        [
            9,
            16,
            19,
        ],
        [
            0,
            9,
            16,
        ],
    ];

    // Process each face (color region)
    for face_vertices in color_vertices.iter_mut() {
        if face_vertices.len() < 3 {
            continue;
        }

        // Sort vertices using topological information instead of angles
        if !sort_vertices_topologically(face_vertices, &vertex_map) {
            continue;
        }

        

        // Add darts for this face. at the end we need the exact number of darts or it will panic
        map.add_free_darts(face_vertices.len());

        // Process each vertex pair to insert and sew the darts
        for i in 0..face_vertices.len() {
            let current_vertex = &face_vertices[i];
            let next_vertex = &face_vertices[(i + 1) % face_vertices.len()];

            // Recording for the beta2 sewing later on

            // Add vertex geometry if not already added
            if !vertices_id.contains_key(current_vertex) {
                vertices_id.insert(current_vertex.to_vec(), dart_id);
                let (x, y) = vertex_map[current_vertex];
                const SCALE: f32 = 5.0;
                let scaled_pos =
                    Vertex2::from((x as f32 * SCALE / res as f32, y as f32 * SCALE / res as f32));
                map.force_write_vertex(dart_id, scaled_pos);
            }

            edges.insert((current_vertex, next_vertex), dart_id);
            // Sew to opposite dart by checking if it exists in the edges map
            if let Some(&opposite_dart) = edges.get(&(next_vertex, current_vertex)) {
                map.force_sew::<2>(opposite_dart, dart_id);
            }
            // Sew to previous dart in face
            if i > 0 {
                map.force_sew::<1>(dart_id - 1, dart_id);
            }

            dart_id += 1;
        }

        // Close the face by sewing first and last darts
        map.force_sew::<1>(dart_id - 1, dart_id - face_vertices.len() as u32);
    }

     //Print the facets and its vertices of the map -- we're looking at face_id 85
/*      println!("Facets in the map:");
     map.iter_faces()
         .for_each(|face_id| {
             println!("Face {}", face_id);
             println!("  Vertices:");
             Orbit2::new(&map, OrbitPolicy::Custom(&[1]), face_id as DartIdType)
                 .for_each(|dart_id| {
                     let vid = map.vertex_id(dart_id);
                     let vertex = map.force_read_vertex(vid).unwrap();
                     println!("    {:?}", vertex);
                 }); */
          /*   if (face_id == 85) {
                println!("Face {}", face_id);
                println!("  Vertices:"); */
            /*     Orbit2::new(&map, OrbitPolicy::Custom(&[1]), face_id as DartIdType)
                    .for_each(|dart_id| {
                        let vid = map.vertex_id(dart_id);
                        let vertex = map.force_read_vertex(vid).unwrap();
                        println!("    {:?}", vertex);
                    }); */
      
/*                 }
            } */

    let mut orbit = Orbit2::new(&map, OrbitPolicy::Custom(&[1]), 188 as DartIdType);
    while let Some(dart_id) = orbit.next() {
        let vid = map.vertex_id(dart_id);
        let vertex = map.force_read_vertex(vid).unwrap();
        println!("    {:?}, {}", vertex, vid);
    };

    let duration = start.elapsed();
    println!("Time elapsed in generate_mesh: {:?}", duration);
    Ok(map)
}

/* #[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sort_vertices_topologically(){
        let mut vertices = vec![vec![0, 21, 24], vec![21, 22,], vec![1, 2, 4]];
        let mut vertex_map = HashMap::new();
        vertex_map.insert(vec![1, 2, 3], (0, 0));
        vertex_map.insert(vec![1, 2, 3, 4], (0, 0));
        vertex_map.insert(vec![1, 2, 4], (0, 0));
        sort_vertices_topologically(&mut vertices, &vertex_map);
        assert_eq!(vertices, vec![vec![1, 2, 3, 4]]);
    } */