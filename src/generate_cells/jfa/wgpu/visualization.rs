use image::{Rgb, RgbImage};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

const RESO: usize = 16; // Match the resolution in mod.rs
const OUTPUT_SIZE: u32 = 640; // Desired output image size
const CELL_SIZE: u32 = OUTPUT_SIZE / RESO as u32; // Size of each cell in the output image

// Use a static Mutex to store the color map between function calls
static COLOR_MAP: Lazy<Mutex<HashMap<u32, Rgb<u8>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn generate_image_visualization(buffer: &[u32], output_path: &str) -> Result<(), String> {
    // Get or generate colors for each point ID
    ensure_color_map_has_all_ids(buffer)?;

    // Create an RGB image with white background at the larger size
    let mut img = RgbImage::from_pixel(OUTPUT_SIZE, OUTPUT_SIZE, Rgb([255, 255, 255]));

    // Lock the color map just once for the duration of the pixel painting
    let color_map = COLOR_MAP
        .lock()
        .map_err(|e| format!("Failed to lock color map: {}", e))?;

    // Draw each cell as a square of CELL_SIZE x CELL_SIZE pixels
    for y in 0..RESO {
        for x in 0..RESO {
            let idx = x + y * RESO;
            let point_id = buffer[idx];

            // Only color cells that have been assigned to a point
            if point_id > 0 {
                if let Some(&color) = color_map.get(&point_id) {
                    // Calculate the top-left corner of this cell in the output image
                    let start_x = x as u32 * CELL_SIZE;
                    let start_y = y as u32 * CELL_SIZE;

                    // Fill the entire cell with the color
                    for cy in 0..CELL_SIZE {
                        for cx in 0..CELL_SIZE {
                            img.put_pixel(start_x + cx, start_y + cy, color);
                        }
                    }
                }
            }
            // cells with point_id == 0 remain white
        }
    }

    // Draw grid lines to make the cell boundaries visible
    draw_grid_lines(&mut img)?;

    // Save the image
    if let Err(e) = img.save(Path::new(output_path)) {
        return Err(format!("Failed to save image: {}", e));
    }

    Ok(())
}

// Helper function to draw grid lines
fn draw_grid_lines(img: &mut RgbImage) -> Result<(), String> {
    let grid_color = Rgb([200, 200, 200]); // Light gray for grid lines

    // Draw horizontal grid lines
    for y in 0..=RESO {
        let y_pos = y as u32 * CELL_SIZE;
        if y_pos >= OUTPUT_SIZE {
            continue;
        }

        for x in 0..OUTPUT_SIZE {
            img.put_pixel(x, y_pos, grid_color);
        }
    }

    // Draw vertical grid lines
    for x in 0..=RESO {
        let x_pos = x as u32 * CELL_SIZE;
        if x_pos >= OUTPUT_SIZE {
            continue;
        }

        for y in 0..OUTPUT_SIZE {
            img.put_pixel(x_pos, y, grid_color);
        }
    }

    Ok(())
}

fn ensure_color_map_has_all_ids(buffer: &[u32]) -> Result<(), String> {
    let mut map = COLOR_MAP
        .lock()
        .map_err(|e| format!("Failed to lock color map: {}", e))?;

    // Find all unique IDs in the buffer that don't yet have a color
    let mut new_ids = Vec::new();
    for &id in buffer {
        if id > 0 && !map.contains_key(&id) {
            new_ids.push(id);
        }
    }

    // If we have new IDs, generate colors for them
    if !new_ids.is_empty() {
        let mut rng = rand::thread_rng();
        use rand::Rng;

        for id in new_ids {
            // Generate a random color for this ID
            // Avoid very light colors for visibility on white background
            let r = rng.gen_range(0..=200);
            let g = rng.gen_range(0..=200);
            let b = rng.gen_range(0..=200);

            // Ensure at least one component is quite dark for contrast with white background
            let min_component = r.min(g).min(b);
            if min_component > 100 {
                // Make one component darker for better contrast
                let component_to_darken = rng.gen_range(0..=2);
                match component_to_darken {
                    0 => map.insert(id, Rgb([r / 3, g, b])),
                    1 => map.insert(id, Rgb([r, g / 3, b])),
                    _ => map.insert(id, Rgb([r, g, b / 3])),
                };
            } else {
                map.insert(id, Rgb([r, g, b]));
            }
        }
    }

    Ok(())
}
