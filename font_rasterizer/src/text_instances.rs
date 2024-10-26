use std::collections::BTreeMap;

use text_buffer::{
    buffer::{BufferChar, CellPosition},
    caret::{Caret, CaretType},
};
use wgpu::{Device, Queue};

use crate::{
    font_buffer::Direction,
    instances::{GlyphInstance, GlyphInstances, InstanceKey},
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct TextInstancesKey {
    c: char,
    position: CellPosition,
}

impl From<BufferChar> for TextInstancesKey {
    fn from(value: BufferChar) -> Self {
        let BufferChar { c, position } = value;
        Self { c, position }
    }
}

// FIXME: 一時的な置き場
#[inline]
pub fn caret_char(caret_type: CaretType) -> char {
    match caret_type {
        CaretType::Primary => '_',
        CaretType::Mark => '^',
    }
}

impl From<Caret> for TextInstancesKey {
    fn from(value: Caret) -> Self {
        let Caret { position, .. } = value;
        Self {
            c: caret_char(value.caret_type),
            position,
        }
    }
}

impl TextInstancesKey {
    pub fn to_instance_key(&self) -> InstanceKey {
        InstanceKey::Position(self.position.row, self.position.col)
    }

    pub fn to_pre_remove_instance_key(&self) -> InstanceKey {
        InstanceKey::PreRemovePosition(self.position.row, self.position.col)
    }

    pub fn same_position(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

#[derive(Default)]
pub struct TextInstances {
    glyph_instances: BTreeMap<char, GlyphInstances>,
    direction: Direction,
}

impl TextInstances {
    pub fn add(&mut self, key: TextInstancesKey, instance: GlyphInstance, device: &Device) {
        let instances = self.glyph_instances.entry(key.c).or_insert_with(|| {
            let mut instances = GlyphInstances::new(key.c, device);
            instances.set_direction(&self.direction);
            instances
        });
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

    pub fn pre_remove(&mut self, key: &TextInstancesKey) {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            if let Some(instance) = instances.remove(&key.to_instance_key()) {
                instances.insert(key.to_pre_remove_instance_key(), instance);
            }
        }
    }

    pub fn get_mut_from_dustbox(&mut self, key: &TextInstancesKey) -> Option<&mut GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.get_mut(&key.to_pre_remove_instance_key())
        } else {
            None
        }
    }

    pub fn remove_from_dustbox(&mut self, key: &TextInstancesKey) -> Option<GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.remove(&key.to_pre_remove_instance_key())
        } else {
            None
        }
    }

    pub fn update(&mut self, device: &Device, queue: &Queue) {
        for instances in self.glyph_instances.values_mut() {
            instances.update_buffer(device, queue)
        }
    }

    pub fn set_direction(&mut self, direction: &Direction) {
        self.direction = *direction;
        for instances in self.glyph_instances.values_mut() {
            instances.set_direction(direction);
        }
    }

    pub fn to_instances(&self) -> Vec<&GlyphInstances> {
        self.glyph_instances.values().collect()
    }

    pub fn len(&self) -> usize {
        self.glyph_instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.glyph_instances.is_empty()
    }

    pub fn clear(&mut self) {
        // GlyphInstances は wgpu::Buffer を保持しているが
        // self.glyph_instances.clear() だと Buffer も破棄されてしまい
        // Buffer の作り直しになってコストが高そうなため
        //  GlyphInstances の内部データのクリアにとどめている
        self.glyph_instances
            .values_mut()
            .for_each(|instances| instances.clear());
    }
}
