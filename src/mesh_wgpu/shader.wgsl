struct Vertex {
    x: u32,
    y: u32,
    colors: array<u32, 4>, // Maximum 4 colors per vertex
    n_colors: u32,
}

@group(0) @binding(0) var<storage, read_write> pixel_grid: array<u32>;
@group(0) @binding(1) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(2) var<uniform> resolution: u32;

fn is_valid_vertex(colors: array<u32, 4>, n_colors: u32) -> bool {
    return n_colors >= 3;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;
    
    if x >= resolution || y >= resolution {
        return;
    }

    let index = y * resolution + x;
    let current_color = pixel_grid[index];
    
    // Collect colors from 8-neighborhood
    var unique_colors: array<u32, 4>;
    var n_colors: u32 = 0;
    
    // Check 8-neighborhood
    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            let nx = i32(x) + dx;
            let ny = i32(y) + dy;
            
            if nx >= 0 && nx < i32(resolution) && ny >= 0 && ny < i32(resolution) {
                let neighbor_color = pixel_grid[u32(ny) * resolution + u32(nx)];
                
                // Add color if not already present
                var is_new = true;
                for (var i = 0u; i < n_colors; i++) {
                    if unique_colors[i] == neighbor_color {
                        is_new = false;
                        break;
                    }
                }
                
                if is_new && n_colors < 4 {
                    unique_colors[n_colors] = neighbor_color;
                    n_colors++;
                }
            }
        }
    }
    
    // If we have 3 or more unique colors, this is a vertex
    if n_colors >= 3 {
        let vertex = Vertex(x, y, unique_colors, n_colors);
        // Store vertex in output buffer
        // Note: Need atomic operations for real implementation
        vertices[index] = vertex;
    }
}
