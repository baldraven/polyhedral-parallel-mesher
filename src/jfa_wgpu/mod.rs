pub mod visualization;
//use visualization::generate_image_visualization;
use std::mem::size_of_val;

pub async fn run(points: &[(f64, f64)], config: (f64, f64), reso: u32) -> Vec<u32> {
    let reso_usize = reso as usize;

    let context = WgpuContext::new(
        reso_usize * reso_usize * std::mem::size_of::<u32>(),
        points.len() * std::mem::size_of::<(u32, u32)>(),
        reso,
    )
    .await;

    let normal_points = init_normal_points(points, config, reso_usize);

    let mut local_buffer = vec![0; reso_usize * reso_usize];

    // Mark the initial points on the grid with their respective color
    for (i, point) in normal_points.iter().enumerate() {
        let color = i + 1; // 0 means uncolored
        local_buffer[point.0 as usize + point.1 as usize * reso_usize] = color as u32;
    }

    // Flatten normal_points
    let normal_points: Vec<u32> = normal_points
        .iter()
        .flat_map(|(x, y)| vec![*x, *y])
        .collect();

    context.queue.write_buffer(
        &context.normal_points,
        0,
        bytemuck::cast_slice(&normal_points),
    );
    context.queue.write_buffer(
        &context.storage_buffer,
        0,
        bytemuck::cast_slice(&local_buffer),
    );
    context
        .queue
        .write_buffer(&context.reso_buffer, 0, bytemuck::cast_slice(&[reso]));

    let mut k = (reso_usize / 2).max(1) as u32;

    log::info!("Starting JFA iterations...");

   // let mut step_count = 2;
    while k >= 1 {
        jfa_step(&context, &mut local_buffer, k, reso_usize).await;
 //       step_count += 1;
        k /= 2;
    }

    log::info!("done!");

    local_buffer
}

/*
async fn visualize_buffer(buffer: &[u32], step_name: &str) {
    log::info!("Visualizing buffer state: {}", step_name);

    // Create a directory for visualization outputs if it doesn't exist
    let vis_dir = "/home/ely/gitlab/blue_noise/visualizations";
    if !std::path::Path::new(vis_dir).exists() {
        std::fs::create_dir_all(vis_dir).expect("Failed to create visualization directory");
    }

    // Simple text-based visualization for debugging
    // Save the buffer state to a file
    let filename = format!("{}/{}.txt", vis_dir, step_name);
    let mut file = std::fs::File::create(&filename).expect("Failed to create visualization file");

    use std::io::Write;
    writeln!(file, "Buffer state at {}", step_name).expect("Failed to write to file");

    // Print grid representation
    for y in 0..RESO {
        for x in 0..RESO {
            let idx = x + y * RESO;
            write!(file, "{:4}", buffer[idx]).expect("Failed to write to file");
        }
        writeln!(file).expect("Failed to write to file");
    }

    // Generate and save image visualization
    let img_filename = format!("{}/{}.png", vis_dir, step_name);
    if let Err(e) = generate_image_visualization(buffer, &img_filename) {
        log::error!("Failed to generate image: {}", e);
    } else {
        log::info!("Image visualization saved to {}", img_filename);
    }

    log::info!("Text visualization saved to {}", filename);
}

*/

async fn jfa_step(context: &WgpuContext, local_buffer: &mut [u32], k: u32, reso: usize) {
    context
        .queue
        .write_buffer(&context.step_buffer, 0, bytemuck::cast_slice(&[k]));

    let mut command_encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut compute_pass = command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&context.pipeline);
        compute_pass.set_bind_group(0, &context.bind_group, &[]);
        compute_pass.dispatch_workgroups((reso as u32 / 16) as u32, (reso as u32 / 16) as u32, 1);
    }

    context.queue.submit(Some(command_encoder.finish()));
    //TODO: don't get data until the end https://github.com/gfx-rs/wgpu/wiki/Do's-and-Dont's
    get_data(
        local_buffer,
        &context.storage_buffer,
        &context.output_staging_buffer,
        &context.device,
        &context.queue,
    )
    .await;
}

async fn get_data<T: bytemuck::Pod>(
    output: &mut [T],
    storage_buffer: &wgpu::Buffer,
    staging_buffer: &wgpu::Buffer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) {
    let mut command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    command_encoder.copy_buffer_to_buffer(
        storage_buffer,
        0,
        staging_buffer,
        0,
        size_of_val(output) as u64,
    );
    queue.submit(Some(command_encoder.finish()));
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = flume::bounded(1);
    buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    device.poll(wgpu::Maintain::wait()).panic_on_timeout();
    receiver.recv_async().await.unwrap().unwrap();
    output.copy_from_slice(bytemuck::cast_slice(&buffer_slice.get_mapped_range()[..]));
    staging_buffer.unmap();
}

fn init_normal_points(points: &[(f64, f64)], config: (f64, f64), reso: usize) -> Vec<(u32, u32)> {
    points
        .iter()
        .map(|(a, b)| {
            let x = ((a * reso as f64 / config.0).min(reso as f64 - 1.0)) as u32;
            let y = ((b * reso as f64 / config.1).min(reso as f64 - 1.0)) as u32;
            (x, y)
        })
        .collect()
}

pub fn main(
    points: &[(f64, f64)],
    config: (f64, f64),
    reso: u32,
) -> Result<Vec<usize>, &'static str> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_nanos()
        .init();
    let a = pollster::block_on(run(points, config, reso));
    Ok(a.into_iter().map(|x| x as usize).collect())
}

struct WgpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    storage_buffer: wgpu::Buffer,
    output_staging_buffer: wgpu::Buffer,
    step_buffer: wgpu::Buffer,
    normal_points: wgpu::Buffer,
    reso_buffer: wgpu::Buffer,
}

impl WgpuContext {
    async fn new(buffer_size: usize, points_size: usize, reso: u32) -> WgpuContext {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let mut limits = wgpu::Limits::default();
        limits.max_buffer_size = 2000 << 20; // 2GiB
        limits.max_storage_buffer_binding_size = 2000 << 20; // 2GiB

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let output_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let step_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<u32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let normal_points = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: points_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let reso_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<u32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Write the resolution to the buffer
        queue.write_buffer(&reso_buffer, 0, bytemuck::cast_slice(&[reso]));

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
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
                    resource: storage_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: step_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: normal_points.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: reso_buffer.as_entire_binding(),
                },
            ],
        });

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

        WgpuContext {
            device,
            queue,
            pipeline,
            bind_group,
            storage_buffer,
            output_staging_buffer,
            step_buffer,
            normal_points,
            reso_buffer,
        }
    }
}
