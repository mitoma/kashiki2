use std::collections::BTreeMap;

use text_buffer::{
    buffer::{BufferChar, CellPosition},
    caret::Caret,
};
use wgpu::{Device, Queue};

use font_rasterizer::{
    font_buffer::Direction,
    instances::{GlyphInstance, GlyphInstances, InstanceKey},
};

use crate::ui::caret_char;

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
    pub(crate) fn to_instance_key(&self) -> InstanceKey {
        InstanceKey::Position(self.position.row, self.position.col)
    }

    pub(crate) fn to_pre_remove_instance_key(&self) -> InstanceKey {
        InstanceKey::PreRemovePosition(self.position.row, self.position.col)
    }
}

#[derive(Default)]
pub(crate) struct TextInstances {
    glyph_instances: BTreeMap<char, GlyphInstances>,
    direction: Direction,
}

impl TextInstances {
    pub(crate) fn add(&mut self, key: TextInstancesKey, instance: GlyphInstance, device: &Device) {
        let instances = self.glyph_instances.entry(key.c).or_insert_with(|| {
            let mut instances = GlyphInstances::new(key.c, device);
            instances.set_direction(&self.direction);
            instances
        });
        instances.insert(key.to_instance_key(), instance)
    }

    pub(crate) fn get_mut(&mut self, key: &TextInstancesKey) -> Option<&mut GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.get_mut(&key.to_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn remove(&mut self, key: &TextInstancesKey) -> Option<GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.remove(&key.to_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn pre_remove(&mut self, key: &TextInstancesKey) {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            if let Some(instance) = instances.remove(&key.to_instance_key()) {
                instances.insert(key.to_pre_remove_instance_key(), instance);
            }
        }
    }

    pub(crate) fn get_mut_from_dustbox(
        &mut self,
        key: &TextInstancesKey,
    ) -> Option<&mut GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.get_mut(&key.to_pre_remove_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn remove_from_dustbox(&mut self, key: &TextInstancesKey) -> Option<GlyphInstance> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.remove(&key.to_pre_remove_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn update(&mut self, device: &Device, queue: &Queue) {
        for instances in self.glyph_instances.values_mut() {
            instances.update_buffer(device, queue)
        }
    }

    pub(crate) fn set_direction(&mut self, direction: &Direction) {
        self.direction = *direction;
        for instances in self.glyph_instances.values_mut() {
            instances.set_direction(direction);
        }
    }

    pub(crate) fn to_instances(&self) -> Vec<&GlyphInstances> {
        self.glyph_instances.values().collect()
    }
}
