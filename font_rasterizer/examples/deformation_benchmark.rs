use std::{
    error::Error,
    sync::{Arc, mpsc},
};

use clap::Parser;
use font_collector::FontCollector;
use font_rasterizer::{
    char_width_calcurator::CharWidth,
    font_converter::convert_char_to_vector_vertices,
    motion::{CameraDetail, EasingFuncType, MotionDetail, MotionFlags, MotionTarget, MotionType},
    rasterizer_pipeline::Buffers,
    rasterizer_renderrer::{OutlineFillRule, RasterizerRenderrer},
    time::{ClockMode, increment_fixed_clock, set_clock_mode},
    vector_instances::{InstanceAttributes, VectorInstances},
    vector_vertex_buffer::VectorVertexBuffer,
};
use glam::{Mat4, Vec3};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    direct: bool,
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u32).range(1..))]
    frames: u32,
    #[arg(long, default_value_t = 20)]
    warmup: u32,
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(1..=300))]
    instances: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    pollster::block_on(run(Args::parse()))
}

async fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = if cfg!(target_os = "windows") {
        wgpu::Backends::DX12
    } else {
        wgpu::Backends::all()
    };
    let instance = wgpu::Instance::new(descriptor);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;
    let features =
        wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    if !adapter.features().contains(features) {
        return Err("GPU timestamps are required".into());
    }
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            ..Default::default()
        })
        .await?;
    let font = FontCollector::default()
        .convert_font(
            include_bytes!("../../fonts/BIZUDMincho-Regular.ttf").to_vec(),
            None,
        )
        .ok_or("invalid font")?;
    let (glyph, _) =
        convert_char_to_vector_vertices(Arc::new(vec![font]), None, 'あ', CharWidth::Wide)?;
    let mut vertices = VectorVertexBuffer::default();
    vertices.append(&device, &queue, "glyph".to_owned(), glyph)?;
    let mut instances = VectorInstances::new("glyph".to_owned(), &device);
    for index in 0..args.instances {
        let detail = match index % 3 {
            0 => MotionDetail::USE_X_DISTANCE,
            1 => MotionDetail::USE_Y_DISTANCE,
            _ => MotionDetail::USE_XY_DISTANCE,
        };
        let target = match index % 4 {
            0 => MotionTarget::ROTATE_Z_PLUS | MotionTarget::STRETCH_X_PLUS,
            1 => MotionTarget::ROTATE_Y_PLUS | MotionTarget::MOVE_Y_PLUS,
            2 => MotionTarget::STRETCH_Y_PLUS | MotionTarget::MOVE_X_MINUS,
            _ => MotionTarget::ROTATE_X_MINUS | MotionTarget::STRETCH_X_MINUS,
        };
        instances.push(InstanceAttributes {
            position: Vec3::new(
                -0.94 + (index % 20) as f32 * 0.096,
                0.90 - (index / 20) as f32 * 0.12,
                0.5,
            ),
            instance_scale: [0.075; 2],
            motion: MotionFlags::new(
                MotionType::EaseInOut(EasingFuncType::Sin, true),
                detail | MotionDetail::TURN_BACK,
                target,
                CameraDetail::empty(),
            ),
            start_time: index % 17,
            gain: 0.6,
            duration: web_time::Duration::from_millis(1024),
            color: [0.25, 0.5, 0.75],
            ..Default::default()
        });
    }
    instances.update_buffer(&device, &queue);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = RasterizerRenderrer::new_with_curve_projection(
        &device,
        512,
        512,
        format,
        true,
        OutlineFillRule::NonZero,
        !args.direct,
    );
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Deformation benchmark"),
        size: wgpu::Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: None,
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolved = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let view = texture.create_view(&Default::default());
    let references = [&instances];
    let matrix = Mat4::IDENTITY.to_cols_array_2d();
    set_clock_mode(ClockMode::Fixed);
    let total = args
        .warmup
        .checked_add(args.frames)
        .ok_or("frame count overflow")?;
    let mut samples = Vec::new();
    for frame in 0..total {
        increment_fixed_clock(web_time::Duration::from_millis(16));
        renderer.prepare(&device, &queue, (matrix, matrix));
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.write_timestamp(&queries, 0);
        renderer.render(
            &mut encoder,
            Buffers {
                glyph_buffers: None,
                vector_buffers: Some((&vertices, &references)),
            },
            &view,
        );
        encoder.write_timestamp(&queries, 1);
        encoder.resolve_query_set(&queries, 0..2, &resolved, 0);
        encoder.copy_buffer_to_buffer(&resolved, 0, &readback, 0, 16);
        queue.submit(Some(encoder.finish()));
        let (sender, receiver) = mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;
        let mapped = readback.slice(..).get_mapped_range().unwrap();
        let start = u64::from_le_bytes(mapped[..8].try_into()?);
        let end = u64::from_le_bytes(mapped[8..].try_into()?);
        let millis =
            end.wrapping_sub(start) as f64 * f64::from(queue.get_timestamp_period()) / 1_000_000.0;
        if frame >= args.warmup {
            samples.push(millis);
        }
        drop(mapped);
        readback.unmap();
    }
    let pixels = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 512 * 512 * 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &pixels,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(512 * 4),
                rows_per_image: Some(512),
            },
        },
        texture.size(),
    );
    queue.submit(Some(encoder.finish()));
    let (sender, receiver) = mpsc::channel();
    pixels
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    let mapped = pixels.slice(..).get_mapped_range().unwrap();
    let lit = mapped
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[3] != 0)
        .count();
    let checksum = mapped.iter().fold(14695981039346656037u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(1099511628211)
    });
    if lit == 0 {
        return Err("blank benchmark image".into());
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "adapter={}, direct={}, instances={}, frames={}, gpu_mean_ms={:.4}, gpu_median_ms={:.4}, lit_pixels={lit}, checksum={checksum:016x}",
        adapter.get_info().name,
        args.direct,
        args.instances,
        args.frames,
        samples.iter().sum::<f64>() / samples.len() as f64,
        samples[samples.len() / 2]
    );
    Ok(())
}
