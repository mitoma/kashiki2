//! CPU-side layouts shared by hosts that run the canonical WGSL shaders.

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OverlapUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub default_view_proj: [[f32; 4]; 4],
    pub time: u32,
    pub width: u32,
    pub enable_antialiasing: u32,
    pub padding: [u32; 1],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 3],
    pub motion: u32,
    pub start_time: u32,
    pub gain: f32,
    pub duration: u32,
}
