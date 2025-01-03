use honeycomb::core::cmap::CMap2;
use honeycomb::core::prelude::{CMapBuilder, Vertex2};
use honeycomb::render::App;
use std::collections::HashMap;

fn is_subset(sub: &[usize], sup: &[usize]) -> bool {
    sub.iter().all(|x| sup.contains(x))
}

/// Remove vertices that are subsets of others
fn remove_subsets(vertices: &mut Vec<Vec<usize>>) {
    let mut i = 0;
    while i < vertices.len() {
        let mut j = 0;
        while j < vertices.len() {
            if i != j && is_subset(&vertices[i], &vertices[j]) {
                vertices.remove(i);
                i = i.saturating_sub(1);
                j = 0;
            } else {
                j += 1;
            }
        }
        i += 1;
    }
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
    for (idx, &current_color) in grid.iter().enumerate() {
        let x = (idx % res) as u32;
        let y = (idx / res) as u32;

        // Collect unique colors in the 8-neighborhood
        // FIXME: A little problem with saturating_sub : lines and column 0 are considered as outside the boundary
        let mut unique_colors: Vec<usize> = [
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
            // Check if neighbor is in bounds or at boundary
            if nx > 0 && ny > 0 && nx < res as u32 && ny < res as u32 {
                grid[ny as usize * res + nx as usize]
            } else {
                0 // Use 0 to represent None/boundary
            }
        })
        .collect();

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
) {
    assert!(vertices.len() >= 3);

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
                assert!(count_common_elements(current, &vertices[0]) == 2);
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

    *vertices = sorted;
}

/// Generates a combinatorial map from a pixel grid by sewing darts between vertices.
/// Each face in the map corresponds to a color region in the pixel grid.
pub fn generate_mesh(pixels: &[usize], num_colors: usize) -> Result<(), &'static str> {
    let res = (pixels.len() as f64).sqrt() as usize;
    let mut color_vertices: Vec<Vec<Vec<usize>>> = vec![Vec::new(); num_colors];
    let mut vertex_map: HashMap<Vec<usize>, (u32, u32)> = HashMap::new();
    let mut vertices_id: HashMap<Vec<usize>, u32> = HashMap::new();
    let mut edges: HashMap<(&[usize], &[usize]), u32> = HashMap::new();

    extract_voronoi_cell_vertices(pixels, res, &mut color_vertices, &mut vertex_map);

    let mut map: CMap2<f32> = CMapBuilder::default().build().unwrap();

    let mut dart_id = 1;

    // Process each face (color region)
    for face_vertices in color_vertices.iter_mut() {
        if face_vertices.len() < 3 {
            continue;
        }

        remove_subsets(face_vertices);

        // Sort vertices using topological information instead of angles
        sort_vertices_topologically(face_vertices, &vertex_map);
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
                let scaled_pos =
                    Vertex2::from((x as f32 * 5.0 / res as f32, y as f32 * 5.0 / res as f32));
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

    //map.iter_faces().collect()

    // Visualize the result
    let mut render_app = App::default();
    render_app.add_capture(&map);
    render_app.run();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_voronoi_cell_vertices() {
        let pixels = vec![
            2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 5, 5, 5, 5, 5, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 5, 5,
            5, 5, 5, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 5, 5, 5, 5, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1,
            1, 1, 5, 5, 5, 5, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 5, 5, 5, 5, 2, 2, 2, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 3, 3, 3, 3, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 3, 2, 2, 1, 1,
            1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 3, 3, 2, 2, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 4,
            4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3,
            3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
            4, 4, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4,
            4, 4, 4, 4, 4, 4, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3,
        ];
        let num_colors = 5;
        let res = (pixels.len() as f64).sqrt() as usize;
        let mut color_vertices: Vec<Vec<Vec<usize>>> = vec![Vec::new(); num_colors];
        let mut vertex_map: HashMap<Vec<usize>, (u32, u32)> = HashMap::new();
        extract_voronoi_cell_vertices(&pixels, res, &mut color_vertices, &mut vertex_map);

        let expected_color_vertices = vec![
            vec![
                vec![0, 1, 2],
                vec![0, 1, 5],
                vec![1, 3, 5],
                vec![1, 2, 4],
                vec![1, 3, 4],
            ],
            vec![vec![0, 1, 2], vec![0, 1, 2, 4], vec![0, 2, 4]],
            vec![vec![1, 3, 5], vec![0, 3, 5], vec![1, 3, 4], vec![0, 3, 4]],
            vec![vec![1, 2, 4], vec![1, 3, 4], vec![0, 2, 4], vec![0, 3, 4]],
            vec![vec![0, 1, 5], vec![1, 3, 5], vec![0, 3, 5]],
        ];

        let mut expected_vertex_map = HashMap::new();
        expected_vertex_map.insert(vec![1, 3, 5], (11, 4));
        expected_vertex_map.insert(vec![1, 2, 4], (2, 7));
        expected_vertex_map.insert(vec![0, 3, 5], (15, 4));
        expected_vertex_map.insert(vec![1, 3, 4], (9, 7));
        expected_vertex_map.insert(vec![0, 1, 2], (4, 0));
        expected_vertex_map.insert(vec![0, 1, 2, 4], (1, 7));
        expected_vertex_map.insert(vec![0, 3, 4], (15, 14));
        expected_vertex_map.insert(vec![0, 2, 4], (0, 8));
        expected_vertex_map.insert(vec![0, 1, 5], (10, 0));

        assert_eq!(color_vertices, expected_color_vertices);
        assert_eq!(vertex_map, expected_vertex_map);
    }
}
