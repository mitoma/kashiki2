pub struct OverlapRecordTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub texture_format: wgpu::TextureFormat,
}

impl OverlapRecordTexture {
    pub fn new(device: &wgpu::Device, size: (u32, u32), label: Option<&str>) -> Self {
        let texture_format = wgpu::TextureFormat::Rgba32Uint;
        let size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            texture_format,
        }
    }
}

pub struct OverlapRecordBuffer {
    pub buffer: wgpu::Buffer,
}

impl OverlapRecordBuffer {
    pub fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Overlap Record Buffer"),
            size: (size.0 * size.1 * 4) as wgpu::BufferAddress, // Assuming 4 bytes per pixel
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    pub fn clear(&self, queue: &wgpu::Queue) {
        let size = self.buffer.size();
        queue.write_buffer(&self.buffer, 0, &vec![0u8; size as usize]);
    }
}
