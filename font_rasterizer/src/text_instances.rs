use std::collections::BTreeMap;

use text_buffer::{buffer::BufferChar, caret::Caret};
use wgpu::{Device, Queue};

use crate::instances::{GlyphInstance, GlyphInstances, InstanceKey};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct TextInstancesKey {
    c: char,
    row: usize,
    col: usize,
}

impl From<BufferChar> for TextInstancesKey {
    fn from(value: BufferChar) -> Self {
        let BufferChar { c, row, col } = value;
        Self { c, row, col }
    }
}

impl From<Caret> for TextInstancesKey {
    fn from(value: Caret) -> Self {
        let Caret { row, col } = value;
        Self { c: '_', row, col }
    }
}

impl TextInstancesKey {
    pub fn to_instance_key(&self) -> InstanceKey {
        InstanceKey::Position(self.row, self.col)
    }

    pub fn same_position(&self, other: &Self) -> bool {
        self.row == other.row && self.col == other.col
    }
}

#[derive(Default)]
pub struct TextInstances {
    glyph_instances: BTreeMap<char, GlyphInstances>,
}

impl TextInstances {
    pub fn add(&mut self, key: TextInstancesKey, instance: GlyphInstance, device: &Device) {
        let instances = self
            .glyph_instances
            .entry(key.c)
            .or_insert_with(|| GlyphInstances::new(key.c, device));
        instances.insert(key.to_instance_key(), instance)
    }

    pub fn get_mut(&mut self, key: &TextInstancesKey) -> Option<&mut GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.get_mut(&key.to_instance_key())
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: &TextInstancesKey) -> Option<GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.remove(&key.to_instance_key())
        } else {
            None
        }
    }

    pub fn update(&mut self, device: &Device, queue: &Queue) {
        for instances in self.glyph_instances.values_mut() {
            instances.update_buffer(device, queue)
        }
    }

    pub fn to_instances(&self) -> Vec<&GlyphInstances> {
        self.glyph_instances.values().collect()
    }
}
