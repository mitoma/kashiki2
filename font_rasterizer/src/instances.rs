use bitflags::bitflags;
use log::info;

bitflags! {
    pub struct MotionFlags: u32 {
        const WAVE_X =   0b_00000000_00000000_00000000_00000001;
        const WAVE_Y =   0b_00000000_00000000_00000000_00000010;
        const WAVE_Z =   0b_00000000_00000000_00000000_00000100;
        const ROTATE_X = 0b_00000000_00000000_00000000_00001000;
        const ROTATE_Y = 0b_00000000_00000000_00000000_00010000;
        const ROTATE_Z = 0b_00000000_00000000_00000000_00100000;
    }
}

pub struct GlyphInstance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    color: [f32; 3],
    motion: MotionFlags,
}

impl GlyphInstance {
    pub fn new(
        position: cgmath::Vector3<f32>,
        rotation: cgmath::Quaternion<f32>,
        color: [f32; 3],
        motion: MotionFlags,
    ) -> Self {
        Self {
            position,
            rotation,
            color,
            motion,
        }
    }
}

impl GlyphInstance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
            color: self.color,
            motion: self.motion.bits,
        }
    }
}

pub struct GlyphInstances {
    pub c: char,
    values: Vec<GlyphInstance>,
    buffer_size: u64,
    buffer: wgpu::Buffer,
    updated: bool,
}

const DEFAULT_BUFFER_UNIT: u64 = 256;

impl GlyphInstances {
    pub fn new(c: char, values: Vec<GlyphInstance>, device: &wgpu::Device) -> Self {
        let buffer_size = (values.len() as u64 / DEFAULT_BUFFER_UNIT) + 1;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Instances Buffer. char:{}", c)),
            size: std::mem::size_of::<InstanceRaw>() as u64 * buffer_size * DEFAULT_BUFFER_UNIT,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            c,
            values,
            buffer_size,
            buffer,
            updated: false,
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn push(&mut self, instance: GlyphInstance) {
        self.updated = true;
        self.values.push(instance)
    }

    pub fn clear(&mut self) {
        self.updated = true;
        self.values.clear();
    }

    pub fn update_buffer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.updated {
            let value_raws: Vec<InstanceRaw> = self.values.iter().map(|v| v.to_raw()).collect();

            // バッファサイズが既存のバッファを上回る場合はバッファを作り直す。
            let buffer_size = (self.values.len() as u64 / DEFAULT_BUFFER_UNIT) + 1;
            if self.buffer_size < buffer_size {
                info!("buffer recreate. char={}, size={}", self.c, buffer_size);
                self.buffer.destroy();
                self.buffer_size = buffer_size;
                self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("Instances Buffer. char:{}", self.c)),
                    size: std::mem::size_of::<InstanceRaw>() as u64
                        * buffer_size
                        * DEFAULT_BUFFER_UNIT,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(value_raws.as_slice()));

            self.updated = false;
        }
    }

    pub fn to_wgpu_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn nth_position(&self, count: usize) -> cgmath::Vector3<f32> {
        self.values[count].position
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 3],
    motion: u32,
}

impl InstanceRaw {
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}
