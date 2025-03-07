use honeycomb::core::cmap::CMap2;
use honeycomb::prelude::{CMapBuilder, Vertex2};
use honeycomb::prelude::{DartIdType, Orbit2, OrbitPolicy};
use honeycomb::render::App;
use std::collections::HashMap;
use std::mem::size_of;
use wgpu::util::DeviceExt;

const WORKGROUP_SIZE: u32 = 16;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    x: u32,
    y: u32,
    colors: [u32; 4],
    n_colors: u32,
}

async fn process_mesh(pixels: &[u32], resolution: u32) -> Vec<Vertex> {
    let context = MeshContext::new(resolution, pixels.len()).await;

    // Write pixel data to GPU
    context
        .queue
        .write_buffer(&context.pixel_buffer, 0, bytemuck::cast_slice(pixels));

    // Create and submit compute pass
    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&context.pipeline);
        compute_pass.set_bind_group(0, &context.bind_group, &[]);

        let workgroup_count = (resolution + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        compute_pass.dispatch_workgroups(workgroup_count, workgroup_count, 1);
    }

    // Get results back from GPU
    let staging_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: context.vertex_buffer.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    encoder.copy_buffer_to_buffer(
        &context.vertex_buffer,
        0,
        &staging_buffer,
        0,
        staging_buffer.size(),
    );
    context.queue.submit(Some(encoder.finish()));

    // Read back the results
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    context
        .device
        .poll(wgpu::Maintain::wait())
        .panic_on_timeout();
    receiver.recv_async().await.unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    let vertices: Vec<Vertex> = bytemuck::cast_slice(&data).to_vec();

    // Filter out invalid vertices (where n_colors < 3)
    vertices.into_iter().filter(|v| v.n_colors >= 3).collect()
}

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

fn _remove_subsets_quadratic(
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
pub async fn run(pixels: &[usize], num_colors: usize) -> Result<(), &'static str> {
    let res = (pixels.len() as f64).sqrt() as usize;

    // Convert pixels to u32 for GPU processing
    let pixels_u32: Vec<u32> = pixels.iter().map(|&x| x as u32).collect();

    // Process vertices on GPU
    let vertices = process_mesh(&pixels_u32, res as u32).await;
    dbg!(&vertices);

    // Convert GPU vertices back to our format
    let mut color_vertices: Vec<Vec<Vec<usize>>> = vec![Vec::new(); num_colors];
    let mut vertex_map: HashMap<Vec<usize>, (u32, u32)> = HashMap::new();

    for vertex in vertices {
        let mut colors: Vec<usize> = vertex.colors[..vertex.n_colors as usize]
            .iter()
            .map(|&x| x as usize)
            .collect();
        colors.sort();

        if !vertex_map.contains_key(&colors) {
            vertex_map.insert(colors.clone(), (vertex.x, vertex.y));
            if vertex.colors[0] > 0 {
                // Skip boundary color (0)
                color_vertices[vertex.colors[0] as usize - 1].push(colors);
            }
        }
    }

    let mut map: CMap2<f32> = CMapBuilder::default().build().unwrap();
    let mut vertices_id: HashMap<Vec<usize>, u32> = HashMap::new();
    let mut edges: HashMap<(&[usize], &[usize]), u32> = HashMap::new();
    let mut dart_id = 1;

    //remove_subsets_quadratic(&mut color_vertices, &mut vertex_map);

    // Process each face (color region)
    for face_vertices in color_vertices.iter_mut() {
        if face_vertices.len() < 3 {
            continue;
        }

        remove_subsets(face_vertices);

        sort_vertices_topologically(face_vertices, &vertex_map);
        map.add_free_darts(face_vertices.len());

        // Process each vertex pair to insert and sew the darts
        for i in 0..face_vertices.len() {
            let current_vertex = &face_vertices[i];
            let next_vertex = &face_vertices[(i + 1) % face_vertices.len()];

            if !vertices_id.contains_key(current_vertex) {
                vertices_id.insert(current_vertex.to_vec(), dart_id);
                let (x, y) = vertex_map[current_vertex];
                let scaled_pos =
                    Vertex2::from((x as f32 * 5.0 / res as f32, y as f32 * 5.0 / res as f32));
                map.force_write_vertex(dart_id, scaled_pos);
            }

            edges.insert((current_vertex, next_vertex), dart_id);
            if let Some(&opposite_dart) = edges.get(&(next_vertex, current_vertex)) {
                map.force_sew::<2>(opposite_dart, dart_id);
            }
            if i > 0 {
                map.force_sew::<1>(dart_id - 1, dart_id);
            }

            dart_id += 1;
        }

        map.force_sew::<1>(dart_id - 1, dart_id - face_vertices.len() as u32);
    }

    //Print the facets and its vertices of the map
    println!("Facets in the map:");
    map.iter_faces().for_each(|face_id| {
        println!("Face {}", face_id);
        println!("  Vertices:");
        Orbit2::new(&map, OrbitPolicy::Custom(&[1]), face_id as DartIdType).for_each(|dart_id| {
            let vid = map.vertex_id(dart_id);
            let vertex = map.force_read_vertex(vid).unwrap();
            println!("    {:?}", vertex);
        });
    });

    // Visualize the result
    let mut render_app = App::default();
    render_app.add_capture(&map);
    render_app.run();

    Ok(())
}

pub fn generate_mesh(pixels: &[usize], num_colors: usize) -> Result<(), &'static str> {
    /*    env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .format_timestamp_nanos()
    .init(); */
    pollster::block_on(run(pixels, num_colors))
}

struct MeshContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    pixel_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
}

impl MeshContext {
    async fn new(resolution: u32, n_pixels: usize) -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // Create buffers
        let pixel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Pixel Grid Buffer"),
            size: (n_pixels * size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (n_pixels * size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let resolution_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Resolution Buffer"),
            contents: bytemuck::cast_slice(&[resolution]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create bind group layout and bind group
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pixel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: resolution_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            device,
            queue,
            pipeline,
            bind_group,
            pixel_buffer,
            vertex_buffer,
        }
    }
}
