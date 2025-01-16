use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
};

use cgmath::{num_traits::ToPrimitive, Rotation3};
use instant::Duration;

use crate::{color_theme::SolarizedColor, motion::MotionFlags, time::now_millis};

#[derive(Clone, Copy, Debug)]
pub struct InstanceAttributes {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub world_scale: [f32; 2],
    pub instance_scale: [f32; 2],
    pub color: [f32; 3],
    pub motion: MotionFlags,
    pub start_time: u32,
    pub gain: f32,
    pub duration: Duration,
}

#[allow(clippy::too_many_arguments)]
impl InstanceAttributes {
    pub fn new(
        position: cgmath::Vector3<f32>,
        rotation: cgmath::Quaternion<f32>,
        world_scale: [f32; 2],
        instance_scale: [f32; 2],
        color: [f32; 3],
        motion: MotionFlags,
        start_time: u32,
        gain: f32,
        duration: Duration,
    ) -> Self {
        Self {
            position,
            rotation,
            world_scale,
            instance_scale,
            color,
            motion,
            start_time,
            gain,
            duration,
        }
    }
}

impl Default for InstanceAttributes {
    fn default() -> Self {
        Self {
            position: [10.0, 0.0, 0.0].into(),
            rotation: cgmath::Quaternion::from_axis_angle(
                cgmath::Vector3::unit_z(),
                cgmath::Deg(0.0),
            ),
            world_scale: [1.0, 1.0],
            instance_scale: [1.0, 1.0],
            color: SolarizedColor::Red.get_color(),
            motion: MotionFlags::ZERO_MOTION,
            start_time: now_millis(),
            gain: 0.0,
            duration: Duration::ZERO,
        }
    }
}

impl InstanceAttributes {
    pub fn random_motion(&mut self) {
        self.start_time = now_millis();
        self.duration = Duration::from_millis(1000);
        self.motion = MotionFlags::random_motion();
        self.gain = 1.0;
    }

    fn as_raw(&self) -> InstanceRaw {
        let model =
            (cgmath::Matrix4::from_nonuniform_scale(self.world_scale[0], self.world_scale[1], 1.0)
                * cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation)
                * cgmath::Matrix4::from_nonuniform_scale(
                    self.instance_scale[0],
                    self.instance_scale[1],
                    1.0,
                ))
            .into();
        InstanceRaw {
            model,
            color: self.color,
            motion: self.motion.into(),
            start_time: self.start_time,
            gain: self.gain,
            duration: self.duration.as_millis().to_u32().unwrap(),
        }
    }
}

// デフォルトのバッファサイズ
// 一つの char に対して256個単位でインスタンス数を持つことができる。
// 小さい数だと wgpu::Buffer の作り直しの頻度が高くなってコスト大きくなり、
// 大きい数だと余分にメモリを消費するためほどほどのサイズにしている。
// 256 は雰囲気で決めた適当な数。
// ひらがなやアルファベットはもっと大きな数が必要になりそうだが
// 漢字などは 256 でも過剰な場合もありそうなので、バランスが難しいところ。
const DEFAULT_BUFFER_UNIT: u64 = 256;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum InstanceKey {
    Monotonic(usize),
    Position(usize, usize),
    // 削除予定だが削除アニメーションの都合でまだ消さない文字のためのキー
    PreRemovePosition(usize, usize),
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 3],
    motion: u32,
    start_time: u32,
    gain: f32,
    duration: u32,
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
                // Action
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Uint32,
                },
                // Action Started At
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Uint32,
                },
                // gain
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 21]>() as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32,
                },
                // duration
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 13,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

pub struct VectorInstances<T> {
    pub key: T,
    values: BTreeMap<InstanceKey, InstanceAttributes>,
    buffer_size: u64,
    buffer: wgpu::Buffer,
    updated: bool,
    monotonic_key: usize,
}

impl<T> VectorInstances<T>
where
    T: Display,
{
    pub fn new(key: T, device: &wgpu::Device) -> Self {
        let values = BTreeMap::new();
        let buffer_size = (values.len() as u64 / DEFAULT_BUFFER_UNIT) + 1;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Instances Buffer. key:{}", key)),
            size: std::mem::size_of::<InstanceRaw>() as u64 * buffer_size * DEFAULT_BUFFER_UNIT,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            key,
            values,
            buffer_size,
            buffer,
            updated: false,
            monotonic_key: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn first(&self) -> Option<&InstanceAttributes> {
        self.values.values().next()
    }

    pub fn get_mut(&mut self, key: &InstanceKey) -> Option<&mut InstanceAttributes> {
        self.updated = true;
        self.values.get_mut(key)
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn push(&mut self, instance: InstanceAttributes) {
        self.updated = true;
        self.values
            .insert(InstanceKey::Monotonic(self.monotonic_key), instance);
        self.monotonic_key += 1;
    }

    pub fn insert(&mut self, key: InstanceKey, instance: InstanceAttributes) {
        self.updated = true;
        self.values.insert(key, instance);
    }

    pub fn remove(&mut self, key: &InstanceKey) -> Option<InstanceAttributes> {
        if let Some(instance) = self.values.remove(key) {
            self.updated = true;
            Some(instance)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.updated = true;
        self.values.clear();
        self.monotonic_key = 0;
    }

    pub fn update_buffer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.updated {
            let value_raws: Vec<InstanceRaw> = self.values.values().map(|v| v.as_raw()).collect();

            // バッファサイズが既存のバッファを上回る場合はバッファを作り直す。
            let buffer_size = (self.values.len() as u64 / DEFAULT_BUFFER_UNIT) + 1;
            if self.buffer_size < buffer_size {
                self.buffer.destroy();
                self.buffer_size = buffer_size;
                self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("Instances Buffer. key:{}", self.key)),
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
}
