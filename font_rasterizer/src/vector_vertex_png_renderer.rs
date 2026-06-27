#![cfg(not(target_arch = "wasm32"))]

use std::{path::Path, sync::mpsc};

use glam::{Mat4, Quat, Vec3};
use image::{ImageBuffer, Rgba};

use crate::{
    errors::FontRasterizerError,
    rasterizer_pipeline::Buffers,
    rasterizer_renderrer::{OutlineFillRule, RasterizerRenderrer},
    vector_instances::{InstanceAttributes, VectorInstances},
    vector_vertex::VectorVertex,
    vector_vertex_buffer::VectorVertexBuffer,
};

pub struct VectorVertexPngRendererOptions {
    pub width: u32,
    pub height: u32,
    pub foreground_color: [f32; 3],
    pub background_color: [u8; 4],
    pub outline_fill_rule: OutlineFillRule,
    pub enable_antialiasing: bool,
}

impl Default for VectorVertexPngRendererOptions {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 1024,
            foreground_color: [0.0, 0.0, 0.0],
            background_color: [255, 255, 255, 255],
            outline_fill_rule: OutlineFillRule::NonZero,
            enable_antialiasing: true,
        }
    }
}

pub fn render_vector_vertex_to_png(
    vector_vertex: VectorVertex,
    output_path: impl AsRef<Path>,
    options: VectorVertexPngRendererOptions,
) -> Result<(), FontRasterizerError> {
    pollster::block_on(render_vector_vertex_to_png_async(
        vector_vertex,
        output_path,
        options,
    ))
}

pub async fn render_vector_vertex_to_png_async(
    vector_vertex: VectorVertex,
    output_path: impl AsRef<Path>,
    options: VectorVertexPngRendererOptions,
) -> Result<(), FontRasterizerError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();

    let mut features = wgpu::Features::empty();
    if options.enable_antialiasing
        && adapter
            .features()
            .contains(wgpu::Features::CONSERVATIVE_RASTERIZATION)
    {
        features |= wgpu::Features::CONSERVATIVE_RASTERIZATION;
    }

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Vector Vertex PNG Renderer Device"),
            required_features: features,
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::default(),
            experimental_features: Default::default(),
        })
        .await
        .unwrap();

    let render_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Vector Vertex Render Texture"),
        size: wgpu::Extent3d {
            width: options.width,
            height: options.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bytes_per_pixel = wgpu::TextureFormat::Rgba8UnormSrgb
        .block_copy_size(None)
        .expect("Rgba8UnormSrgb should have fixed block size");
    let unpadded_bytes_per_row = bytes_per_pixel * options.width;
    let padded_bytes_per_row = (unpadded_bytes_per_row + wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1)
        .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

    let output_buffer_size = padded_bytes_per_row as u64 * options.height as u64;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Vector Vertex Output Buffer"),
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut vector_vertex_buffer = VectorVertexBuffer::new();
    vector_vertex_buffer.append(&device, &queue, "test".to_string(), vector_vertex)?;

    let mut vector_instances = VectorInstances::new("test".to_string(), &device);
    vector_instances.push(InstanceAttributes {
        position: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        world_scale: [1.0, 1.0],
        instance_scale: [1.0, 1.0],
        color: options.foreground_color,
        motion: crate::motion::MotionFlags::ZERO_MOTION,
        start_time: 0,
        gain: 0.0,
        duration: web_time::Duration::ZERO,
    });
    vector_instances.update_buffer(&device, &queue);

    let mut rasterizer = RasterizerRenderrer::new(
        &device,
        options.width,
        options.height,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        options.enable_antialiasing,
        options.outline_fill_rule,
    );

    rasterizer.prepare(
        &device,
        &queue,
        (
            Mat4::IDENTITY.to_cols_array_2d(),
            Mat4::IDENTITY.to_cols_array_2d(),
        ),
    );

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Vector Vertex PNG Render Encoder"),
    });

    let vector_instances_refs = [&vector_instances];
    rasterizer.render(
        &mut encoder,
        Buffers {
            glyph_buffers: None,
            vector_buffers: Some((&vector_vertex_buffer, &vector_instances_refs)),
        },
        &render_view,
    );

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &render_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(options.height),
            },
        },
        wgpu::Extent3d {
            width: options.width,
            height: options.height,
            depth_or_array_layers: 1,
        },
    );

    let submission_index = queue.submit(Some(encoder.finish()));

    let output_buffer_slice = output_buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    output_buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });

    let _ = device
        .poll(wgpu::wgt::PollType::Wait {
            submission_index: Some(submission_index),
            timeout: None,
        })
        .unwrap();

    rx.recv().unwrap().unwrap();

    let data = output_buffer_slice.get_mapped_range();
    let raw_data = if padded_bytes_per_row == unpadded_bytes_per_row {
        data.to_vec()
    } else {
        let mut result = Vec::with_capacity((unpadded_bytes_per_row * options.height) as usize);
        for row in 0..options.height {
            let offset = (row * padded_bytes_per_row) as usize;
            result.extend_from_slice(&data[offset..offset + unpadded_bytes_per_row as usize]);
        }
        result
    };
    drop(data);
    output_buffer.unmap();

    // outline_stage は LoadOp::Clear(TRANSPARENT) で書き込むため、
    // GPU 側ではバックグラウンドカラーを合成できない。
    // 読み出し後にソフトウェアでアルファ合成する。
    let bg = options.background_color;
    let width = options.width;
    let raw_data: Vec<u8> = raw_data
        .chunks_exact(4)
        .enumerate()
        .flat_map(|(i, pixel)| {
            let a = pixel[3] as f32 / 255.0;
            let inv = 1.0 - a;
            let mut composed = [
                (pixel[0] as f32 * a + bg[0] as f32 * inv) as u8,
                (pixel[1] as f32 * a + bg[1] as f32 * inv) as u8,
                (pixel[2] as f32 * a + bg[2] as f32 * inv) as u8,
                (a * 255.0 + bg[3] as f32 * inv) as u8,
            ];

            // 10 ピクセルごとに薄いグリッド線を重ねる。
            let x = i as u32 % width;
            let y = i as u32 / width;
            if x % 10 == 0 || y % 10 == 0 {
                const GRID_ALPHA: f32 = 0.15;
                const GRID_COLOR: [u8; 3] = [128, 128, 128];
                for c in 0..3 {
                    composed[c] = (composed[c] as f32 * (1.0 - GRID_ALPHA)
                        + GRID_COLOR[c] as f32 * GRID_ALPHA)
                        as u8;
                }
            }

            composed
        })
        .collect();

    let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(options.width, options.height, raw_data)
        .unwrap();
    if let Some(parent) = output_path.as_ref().parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    image.save(output_path.as_ref()).unwrap();

    Ok(())
}
