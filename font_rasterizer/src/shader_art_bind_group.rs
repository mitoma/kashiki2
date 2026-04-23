use bytemuck::{Pod, Zeroable};

/// シェーダーアート用のユニフォームバッファ。
/// time / resolution を毎フレーム GPU に送る。
pub struct ShaderArtUniformBuffer {
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ShaderArtUniforms {
    time: f32,
    resolution_width: f32,
    resolution_height: f32,
    // vec4<f32> の 16 バイトアライメントを合わせるためのパディング
    _padding: f32,
    background_color: [f32; 4],
}

impl ShaderArtUniformBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shader Art Uniform Buffer"),
            size: std::mem::size_of::<ShaderArtUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shader Art Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        Self {
            buffer,
            bind_group_layout,
        }
    }

    pub fn update(
        &self,
        queue: &wgpu::Queue,
        time_secs: f32,
        resolution_width: f32,
        resolution_height: f32,
        background_color: [f32; 4],
    ) {
        let uniforms = ShaderArtUniforms {
            time: time_secs,
            resolution_width,
            resolution_height,
            _padding: 0.0,
            background_color,
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn to_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shader Art Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.buffer.as_entire_binding(),
            }],
        })
    }
}
