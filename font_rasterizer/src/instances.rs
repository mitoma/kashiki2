use wgpu::util::DeviceExt;

pub struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    pub fn new(position: cgmath::Vector3<f32>, rotation: cgmath::Quaternion<f32>) -> Self {
        Self { position, rotation }
    }
}

pub struct Instances {
    pub(crate) c: char,
    values: Vec<Instance>,
}

impl Instances {
    pub fn new(c: char, values: Vec<Instance>) -> Self {
        Self { c, values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn push(&mut self, instance: Instance) {
        self.values.push(instance)
    }

    pub fn to_wgpu_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        let value_raws: Vec<InstanceRaw> = self.values.iter().map(|v| v.to_raw()).collect();

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instances Buffer"),
            contents: bytemuck::cast_slice(value_raws.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32; 4]; 4],
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
            ],
        }
    }
}