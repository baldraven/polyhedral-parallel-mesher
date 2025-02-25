@group(0) @binding(0) var<storage, read_write> pixel_grid: array<u32>;
@group(0) @binding(1) var<uniform> step: u32;
@group(0) @binding(2) var<storage, read> normal_points: array<u32>;

const RESO: u32 = 2048;

fn metric(x1: u32, y1: u32, x2: u32, y2: u32) -> u32 {
    let dx = (x1 - x2) * (x1 - x2);
    let dy = (y1 - y2) * (y1 - y2);
    return dx + dy;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    if (x >= RESO || y >= RESO) {
        return;
    }

    let index: u32 = x + y * RESO;
    var current_color = pixel_grid[index];
    let initial_position = index;

    for (var dx = -1; dx <= 1; dx = dx + 1) {
        for (var dy = -1; dy <= 1; dy = dy + 1) {

            //TODO: type checking with if to stay with u32?
            let new_x = u32(i32(x) + dx * i32(step));
            let new_y = u32(i32(y) + dy * i32(step));

            if !(new_x >= 0 && new_x < RESO && new_y >= 0 && new_y < RESO) {
                continue;
            }

            let new_position: u32 = (new_x) + (new_y) * RESO;
            let found_color = pixel_grid[new_position];
            current_color = pixel_grid[initial_position];

            if (dx == 0 && dy == 0) || found_color == 0 || current_color == found_color {
                continue;
            }

            if current_color == 0 {
                pixel_grid[initial_position] = found_color;
                continue;
            }

            // Assign the closest color to the current pixel
            let point1_x = normal_points[(current_color - 1)*2];
            let point2_x = normal_points[(found_color - 1)*2];
            let point1_y = normal_points[(current_color - 1)*2 + 1];
            let point2_y = normal_points[(found_color - 1)*2 + 1];

            let dist1 = f32(((x  - point1_x) * (x  - point1_x ) +
                             (y  - point1_y ) * (y  - point1_y )));
            let dist2 = f32(((x  - point2_x ) * (x  - point2_x ) +
                             (y  - point2_y ) * (y  - point2_y )));

            if dist2 < dist1 {
                pixel_grid[initial_position] = found_color;
            }
        }
    }
}
