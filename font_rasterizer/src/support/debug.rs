use std::collections::HashSet;

use cgmath::Rotation3;
use instant::Duration;
use log::debug;

use crate::{
    camera::Camera,
    font_buffer::GlyphVertexBuffer,
    instances::{GlyphInstance, GlyphInstances},
    motion::MotionFlags,
    rasterizer_pipeline::RasterizerPipeline,
    time::now_millis,
};

const FONT_DATA: &[u8] = include_bytes!("../../examples/font/HackGenConsole-Regular.ttf");

pub async fn run() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(&Default::default(), None)
        .await
        .unwrap();

    let width = 512u32;
    let height = 512u32;

    //let texture_size = 512u32;
    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    };
    let texture = device.create_texture(&texture_desc);
    let texture_view = texture.create_view(&Default::default());

    // we need to store this for later
    let u32_size = std::mem::size_of::<u32>() as u32;

    let output_buffer_size = (u32_size * width * height) as wgpu::BufferAddress;
    let output_buffer_desc = wgpu::BufferDescriptor {
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST
            // this tells wpgu that we want to read this buffer from the cpu
            | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    };
    let output_buffer = device.create_buffer(&output_buffer_desc);

    let font_binaries = vec![FONT_DATA.to_vec()];
    let mut glyph_vertex_buffer = GlyphVertexBuffer::new(font_binaries);

    let mut chars = HashSet::new();
    chars.insert('あ');
    glyph_vertex_buffer
        .append_glyph(&device, &queue, chars)
        .unwrap();

    let camera = Camera::basic((width, height));

    debug!("{:?}", camera.build_view_projection_matrix());

    let instance = GlyphInstance::new(
        (0.0, 0.0, 0.0).into(),
        cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(10.0)),
        [0.4620770514, 0.2501583695, 0.4],
        MotionFlags::ZERO_MOTION,
        now_millis(),
        1.0,
        Duration::from_millis(0),
    );

    let mut instances = GlyphInstances::new('あ', vec![], &device);
    instances.push(instance);
    instances.update_buffer(&device, &queue);
    let i2 = [&instances];

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut pipeline = RasterizerPipeline::new(
        &device,
        width,
        height,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        crate::rasterizer_pipeline::Quarity::VeryHigh,
        wgpu::Color {
            r: 0.0,
            g: 0.3,
            b: 0.3,
            a: 0.3,
        },
    );

    pipeline.run_all_stage(
        &mut encoder,
        &device,
        &queue,
        &glyph_vertex_buffer,
        camera.build_view_projection_matrix().into(),
        &i2,
        texture_view,
    );

    let bytes_per_row = u32_size * width;
    let adjusted_bytes_per_row = if bytes_per_row % 256 == 0 {
        bytes_per_row
    } else {
        bytes_per_row - (bytes_per_row % 256) + 256
    };

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            aspect: wgpu::TextureAspect::All,
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        texture_desc.size,
    );

    queue.submit(Some(encoder.finish()));

    // We need to scope the mapping variables so that we can
    // unmap the buffer
    {
        let buffer_slice = output_buffer.slice(..);

        // NOTE: We have to create the mapping THEN device.poll() before await
        // the future. Otherwise the application will freeze.
        // let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();

        use image::{ImageBuffer, Rgba};
        let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, data).unwrap();
        buffer.save("image.png").unwrap();
    }
    output_buffer.unmap();
}
