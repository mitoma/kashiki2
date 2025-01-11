use crate::{
    glyph_vertex_buffer::Direction,
    vector_instances::{InstanceAttributes, InstanceKey, VectorInstances},
};

pub struct GlyphInstances {
    pub c: char,
    pub direction: Direction,
    instances: VectorInstances<char>,
}

impl GlyphInstances {
    pub fn new(c: char, device: &wgpu::Device) -> Self {
        Self {
            c,
            direction: Direction::Horizontal,
            instances: VectorInstances::new(c, device),
        }
    }

    pub fn set_direction(&mut self, direction: &Direction) {
        self.direction = *direction;
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn first(&self) -> Option<&InstanceAttributes> {
        self.instances.first()
    }

    pub fn get_mut(&mut self, key: &InstanceKey) -> Option<&mut InstanceAttributes> {
        self.instances.get_mut(key)
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn push(&mut self, instance: InstanceAttributes) {
        self.instances.push(instance);
    }

    pub fn insert(&mut self, key: InstanceKey, instance: InstanceAttributes) {
        self.instances.insert(key, instance);
    }

    pub fn remove(&mut self, key: &InstanceKey) -> Option<InstanceAttributes> {
        self.instances.remove(key)
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub fn update_buffer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.instances.update_buffer(device, queue);
    }

    pub fn to_wgpu_buffer(&self) -> &wgpu::Buffer {
        self.instances.to_wgpu_buffer()
    }
}
