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
    let attributes = InstanceAttributes {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        color: options.foreground_color,
        start_time: 0,
        ..Default::default()
    };
    render_with_transform(
        vector_vertex,
        output_path,
        options,
        attributes,
        Mat4::IDENTITY,
    )
    .await
}

async fn render_with_transform(
    vector_vertex: VectorVertex,
    output_path: impl AsRef<Path>,
    options: VectorVertexPngRendererOptions,
    attributes: InstanceAttributes,
    view_proj: Mat4,
) -> Result<(), FontRasterizerError> {
    // headless(サーフェスなし)描画時、Windows の Vulkan ドライバでは overlap ステージの
    // 描画コマンドがラスタライズされない既知の不具合があるため、Windows では DX12 を強制する。
    let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    if cfg!(target_os = "windows") {
        instance_descriptor.backends = wgpu::Backends::DX12;
    }
    let instance = wgpu::Instance::new(instance_descriptor);

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Vector Vertex PNG Renderer Device"),
            required_features: wgpu::Features::empty(),
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
    vector_instances.push(attributes);
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
            view_proj.to_cols_array_2d(),
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

    let data = output_buffer_slice.get_mapped_range().unwrap();
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
        .as_chunks::<4>()
        .0
        .iter()
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
            if x.is_multiple_of(10) || y.is_multiple_of(10) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        VectorVertexBuilder,
        motion::{CameraDetail, MotionDetail, MotionFlags, MotionTarget, MotionType},
    };

    fn local(pixel: f32) -> f32 {
        pixel / 32.0 - 1.0
    }

    fn rectangle(builder: &mut VectorVertexBuilder, bounds: [f32; 4], reverse: bool) {
        let [left, top, right, bottom] = bounds;
        let points = if reverse {
            [[left, top], [left, bottom], [right, bottom], [right, top]]
        } else {
            [[left, top], [right, top], [right, bottom], [left, bottom]]
        };
        builder.move_to(local(points[0][0]), -local(points[0][1]));
        for point in &points[1..] {
            builder.line_to(local(point[0]), -local(point[1]));
        }
        builder.close();
    }

    fn render(
        builder: VectorVertexBuilder,
        rule: OutlineFillRule,
        antialiasing: bool,
        attributes: InstanceAttributes,
        view: Mat4,
    ) -> image::RgbaImage {
        let path =
            std::env::temp_dir().join(format!("kashiki-windfoil-{}.png", std::process::id()));
        pollster::block_on(render_with_transform(
            builder.build(),
            &path,
            VectorVertexPngRendererOptions {
                width: 64,
                height: 64,
                foreground_color: attributes.color,
                background_color: [0; 4],
                outline_fill_rule: rule,
                enable_antialiasing: antialiasing,
            },
            attributes,
            view,
        ))
        .unwrap();
        let image = image::open(&path).unwrap().to_rgba8();
        std::fs::remove_file(path).unwrap();
        image
    }

    fn overlap(pixel: u32, low: f32, high: f32) -> f32 {
        ((pixel as f32 + 1.0).min(high) - (pixel as f32).max(low)).max(0.0)
    }

    fn assert_rectangle(image: &image::RgbaImage, bounds: [f32; 4]) {
        for (horizontal, vertical, pixel) in image.enumerate_pixels() {
            let expected =
                overlap(horizontal, bounds[0], bounds[2]) * overlap(vertical, bounds[1], bounds[3]);
            let actual = pixel[3] as f32 / 255.0;
            assert!(
                (actual - expected).abs() <= 1.1 / 255.0,
                "pixel ({horizontal}, {vertical}): {actual} != {expected}"
            );
        }
    }

    #[test]
    #[ignore = "requires a WebGPU adapter"]
    fn windfoil_gpu_coverage_regressions() {
        let attributes = InstanceAttributes {
            position: Vec3::ZERO,
            start_time: 0,
            color: [0.25, 0.5, 0.75],
            ..Default::default()
        };
        for bounds in [[17.25, 18.75, 44.75, 45.25], [17.25, 18.75, 17.5, 45.25]] {
            let mut builder = VectorVertexBuilder::new();
            rectangle(&mut builder, bounds, false);
            assert_rectangle(
                &render(
                    builder,
                    OutlineFillRule::NonZero,
                    true,
                    attributes,
                    Mat4::IDENTITY,
                ),
                bounds,
            );
        }

        for rule in [OutlineFillRule::NonZero, OutlineFillRule::EvenOdd] {
            for reverse in [false, true] {
                let mut builder = VectorVertexBuilder::new();
                rectangle(&mut builder, [8.0, 8.0, 56.0, 56.0], false);
                rectangle(&mut builder, [24.0, 24.0, 40.0, 40.0], reverse);
                let image = render(builder, rule, true, attributes, Mat4::IDENTITY);
                assert_eq!(image.get_pixel(16, 16)[3], 255);
                assert_eq!(
                    image.get_pixel(32, 32)[3],
                    if reverse || rule == OutlineFillRule::EvenOdd {
                        0
                    } else {
                        255
                    }
                );
                assert_eq!(image.get_pixel(0, 0)[3], 0);
            }
        }

        let mut builder = VectorVertexBuilder::new();
        for stripe in 0..20 {
            let top = 2.25 + stripe as f32 * 3.0;
            rectangle(&mut builder, [11.25, top, 52.75, top + 0.5], false);
        }
        let image = render(
            builder,
            OutlineFillRule::NonZero,
            true,
            attributes,
            Mat4::IDENTITY,
        );
        for (horizontal, vertical, pixel) in image.enumerate_pixels() {
            let expected: f32 = (0..20)
                .map(|stripe| {
                    let top = 2.25 + stripe as f32 * 3.0;
                    overlap(horizontal, 11.25, 52.75) * overlap(vertical, top, top + 0.5)
                })
                .sum();
            assert!((pixel[3] as f32 / 255.0 - expected).abs() <= 1.1 / 255.0);
        }

        let mut builder = VectorVertexBuilder::new();
        builder.move_to(local(16.0), -local(32.0));
        builder.quad_to(local(32.0), -local(0.0), local(48.0), -local(32.0));
        builder.line_to(local(48.0), -local(48.0));
        builder.line_to(local(16.0), -local(48.0));
        builder.close();
        let image = render(
            builder,
            OutlineFillRule::NonZero,
            true,
            attributes,
            Mat4::IDENTITY,
        );
        for (horizontal, vertical, pixel) in image.enumerate_pixels() {
            let mut inside = 0u32;
            for sample_y in 0..64 {
                for sample_x in 0..64 {
                    let sample_x = horizontal as f64 + (sample_x as f64 + 0.5) / 64.0;
                    let sample_y = vertical as f64 + (sample_y as f64 + 0.5) / 64.0;
                    let top = 16.0 + (sample_x - 32.0).powi(2) / 16.0;
                    inside += u32::from(
                        (16.0..48.0).contains(&sample_x) && sample_y >= top && sample_y < 48.0,
                    );
                }
            }
            let expected = inside as f32 / 4096.0;
            assert!(
                (pixel[3] as f32 / 255.0 - expected).abs() < 0.005,
                "quadratic ({horizontal}, {vertical})"
            );
        }

        let mut builder = VectorVertexBuilder::new();
        rectangle(&mut builder, [24.25, 24.25, 39.75, 39.75], false);
        let motion = MotionFlags::new(
            MotionType::None,
            MotionDetail::USE_X_DISTANCE,
            MotionTarget::MOVE_X_PLUS,
            CameraDetail::empty(),
        );
        let transformed = InstanceAttributes {
            motion,
            gain: 0.5,
            ..attributes
        };
        assert_rectangle(
            &render(
                builder,
                OutlineFillRule::NonZero,
                true,
                transformed,
                Mat4::IDENTITY,
            ),
            [20.375, 24.25, 43.625, 39.75],
        );

        let mut builder = VectorVertexBuilder::new();
        rectangle(&mut builder, [17.25, 18.75, 44.75, 45.25], false);
        let image = render(
            builder,
            OutlineFillRule::NonZero,
            false,
            attributes,
            Mat4::IDENTITY,
        );
        assert!(image.pixels().all(|pixel| pixel[3] == 0 || pixel[3] == 255));

        let image = render(
            VectorVertexBuilder::new(),
            OutlineFillRule::NonZero,
            true,
            attributes,
            Mat4::IDENTITY,
        );
        assert!(image.pixels().all(|pixel| pixel[3] == 0));
    }
}
