const RESO: usize = 2048;

pub async fn run(points: &[(f64, f64)], config: (f64, f64)) -> Vec<u32> {
    let context = WgpuContext::new(
        RESO * RESO * std::mem::size_of::<u32>(),
        points.len() * std::mem::size_of::<(u32, u32)>(),
    )
    .await;

    let normal_points = init_normal_points(points, config);

    let mut local_buffer = vec![0; RESO * RESO];

    // Mark the initial points on the grid with their respective color
    for (i, point) in normal_points.iter().enumerate() {
        let color = i + 1; // 0 means uncolored
        local_buffer[point.0 as usize + point.1 as usize * RESO] = color as u32;
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

    let mut k = (RESO / 2).max(1) as u32;

    log::info!("Starting JFA iterations...");

    jfa_step(&context, &mut local_buffer, 1).await;
    while k >= 1 {
        jfa_step(&context, &mut local_buffer, k).await;
        k /= 2;
    }

    log::info!("done!");

    local_buffer
}

async fn jfa_step(context: &WgpuContext, local_buffer: &mut [u32], k: u32) {
    //log::info!("Dispatching JFA step with k = {}", k);

    context.queue.write_buffer(
        &context.storage_buffer,
        0,
        bytemuck::cast_slice(local_buffer),
    );

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
        compute_pass.dispatch_workgroups((RESO / 16) as u32, (RESO / 16) as u32, 1);
    }

    command_encoder.copy_buffer_to_buffer(
        &context.storage_buffer,
        0,
        &context.output_staging_buffer,
        0,
        context.storage_buffer.size(),
    );

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

fn init_normal_points(points: &[(f64, f64)], config: (f64, f64)) -> Vec<(u32, u32)> {
    points
        .iter()
        .map(|(a, b)| {
            let x = ((a * RESO as f64 / config.0).min(RESO as f64 - 1.0)) as u32;
            let y = ((b * RESO as f64 / config.1).min(RESO as f64 - 1.0)) as u32;
            (x, y)
        })
        .collect()
}

pub fn main(points: &[(f64, f64)], config: (f64, f64)) -> Result<Vec<usize>, &'static str> {
    /*     env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .format_timestamp_nanos()
    .init(); */
    let a = pollster::block_on(run(points, config));
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
}

impl WgpuContext {
    async fn new(buffer_size: usize, points_size: usize) -> WgpuContext {
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
            mapped_at_creation: false, //TODO: usage ?
        });

        let normal_points = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: points_size as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
        }
    }
}

/* #[cfg(test)]
mod tests;
 */
