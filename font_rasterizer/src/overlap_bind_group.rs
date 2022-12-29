//use std::time::SystemTime;

use cgmath::SquareMatrix;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    view_proj: [[f32; 4]; 4],
    time: f32,
    // padding が必要らしい。正直意味わかんねぇな。
    padding: [u32; 3],
}

/// オーバーラップ用の BindGroup。
/// Uniforms として現在時刻のみ渡している。
///
/// 今後の展望
/// カメラ位置などを乗せたいですねぇ。
pub struct OverlapBindGroup {
    uniforms: Uniforms,
    pub(crate) layout: wgpu::BindGroupLayout,
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            time: 0.0,
            //            time: SystemTime::now()
            //                .duration_since(SystemTime::UNIX_EPOCH)
            //                .unwrap_or_default()
            //                .as_millis() as f32
            //                / 100.0,
            padding: [0; 3],
        }
    }
}

impl OverlapBindGroup {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Overlap Bind Group Layout"),
        });

        let uniforms = Uniforms::default();

        Self { uniforms, layout }
    }

    pub fn update(&mut self) {
        //        let d = SystemTime::now()
        //            .duration_since(SystemTime::UNIX_EPOCH)
        //            .unwrap_or_default();
        //        self.uniforms.time = (d.as_millis() % 10000000) as f32 / 1000.0;
    }

    pub fn to_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[self.uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Overlap Bind Group"),
        })
    }
}
