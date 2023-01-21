use cgmath::{num_traits::ToPrimitive, SquareMatrix};
use instant::SystemTime;
use wgpu::util::DeviceExt;

use crate::time::now_millis;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    view_proj: [[f32; 4]; 4],
    time: u32,
    // padding が必要らしい。正直意味わかんねぇな。
    padding: [u32; 3],
}

/// オーバーラップ用の BindGroup。
/// Uniforms として現在時刻のみ渡している。
pub struct OverlapBindGroup {
    uniforms: Uniforms,
    buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: wgpu::BindGroupLayout,
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            time: now_millis(),
            padding: [0; 3],
        }
    }
}

impl OverlapBindGroup {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Overlap Bind Group Layout"),
        });

        let uniforms = Uniforms::default();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Overlap Bind Group"),
        });

        Self {
            uniforms,
            buffer,
            bind_group,
            layout,
        }
    }

    pub fn update(&mut self, view_proj: [[f32; 4]; 4]) {
        self.uniforms.view_proj = view_proj;
        let d = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        self.uniforms.time = now_millis();
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniforms]))
    }
}
