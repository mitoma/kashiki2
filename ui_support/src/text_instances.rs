use std::collections::BTreeMap;

use text_buffer::{
    buffer::{BufferChar, CellPosition},
    caret::Caret,
};
use wgpu::{Device, Queue};

use font_rasterizer::{
    glyph_instances::GlyphInstances,
    glyph_vertex_buffer::Direction,
    vector_instances::{InstanceAttributes, InstanceKey, VectorInstances},
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
    pub(crate) fn add(
        &mut self,
        key: TextInstancesKey,
        instance: InstanceAttributes,
        device: &Device,
    ) {
        let instances = self.glyph_instances.entry(key.c).or_insert_with(|| {
            let mut instances = GlyphInstances::new(key.c, device);
            instances.set_direction(&self.direction);
            instances
        });
        instances.insert(key.to_instance_key(), instance)
    }

    pub(crate) fn get_mut(&mut self, key: &TextInstancesKey) -> Option<&mut InstanceAttributes> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.get_mut(&key.to_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn remove(&mut self, key: &TextInstancesKey) -> Option<InstanceAttributes> {
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
    ) -> Option<&mut InstanceAttributes> {
        if let Some(instances) = self.glyph_instances.get_mut(&key.c) {
            instances.get_mut(&key.to_pre_remove_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn remove_from_dustbox(
        &mut self,
        key: &TextInstancesKey,
    ) -> Option<InstanceAttributes> {
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

#[derive(Default)]
pub(crate) struct CaretInstances {
    vector_instances: BTreeMap<String, VectorInstances<String>>,
}

impl CaretInstances {
    pub(crate) fn add(
        &mut self,
        key: TextInstancesKey,
        instance: InstanceAttributes,
        device: &Device,
    ) {
        let instances = self
            .vector_instances
            .entry(key.c.to_string())
            .or_insert_with(|| VectorInstances::new(key.c.to_string(), device));
        instances.insert(key.to_instance_key(), instance)
    }

    pub(crate) fn get_mut(&mut self, key: &TextInstancesKey) -> Option<&mut InstanceAttributes> {
        if let Some(instances) = self.vector_instances.get_mut(&key.c.to_string()) {
            instances.get_mut(&key.to_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn remove(&mut self, key: &TextInstancesKey) -> Option<InstanceAttributes> {
        if let Some(instances) = self.vector_instances.get_mut(&key.c.to_string()) {
            instances.remove(&key.to_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn pre_remove(&mut self, key: &TextInstancesKey) {
        if let Some(instances) = self.vector_instances.get_mut(&key.c.to_string()) {
            if let Some(instance) = instances.remove(&key.to_instance_key()) {
                instances.insert(key.to_pre_remove_instance_key(), instance);
            }
        }
    }

    pub(crate) fn get_mut_from_dustbox(
        &mut self,
        key: &TextInstancesKey,
    ) -> Option<&mut InstanceAttributes> {
        if let Some(instances) = self.vector_instances.get_mut(&key.c.to_string()) {
            instances.get_mut(&key.to_pre_remove_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn remove_from_dustbox(
        &mut self,
        key: &TextInstancesKey,
    ) -> Option<InstanceAttributes> {
        if let Some(instances) = self.vector_instances.get_mut(&key.c.to_string()) {
            instances.remove(&key.to_pre_remove_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn update(&mut self, device: &Device, queue: &Queue) {
        for instances in self.vector_instances.values_mut() {
            instances.update_buffer(device, queue)
        }
    }

    pub(crate) fn to_instances(&self) -> Vec<&VectorInstances<String>> {
        self.vector_instances.values().collect()
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) enum BorderType {
    // │
    Vertical,
    // ─
    Horizontal,
    // ┌
    TopLeft,
    // ┐
    TopRight,
    // └
    BottomLeft,
    // ┘
    BottomRight,
    // ├
    LeftVertical,
    // ┤
    RightVertical,
    // ┬
    TopHorizontal,
    // ┴
    BottomHorizontal,
    // ┼
    CenterHorizontalVertical,
}

impl BorderType {
    pub(crate) fn to_key(&self) -> String {
        match self {
            BorderType::Vertical => "vertical".to_string(),
            BorderType::Horizontal => "horizontal".to_string(),
            BorderType::TopLeft => "top_left".to_string(),
            BorderType::TopRight => "top_right".to_string(),
            BorderType::BottomLeft => "bottom_left".to_string(),
            BorderType::BottomRight => "bottom_right".to_string(),
            BorderType::LeftVertical => "left_vertical".to_string(),
            BorderType::RightVertical => "right_vertical".to_string(),
            BorderType::TopHorizontal => "top_horizontal".to_string(),
            BorderType::BottomHorizontal => "bottom_horizontal".to_string(),
            BorderType::CenterHorizontalVertical => "center_horizontal_vertical".to_string(),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct BorderFragment {
    pub(crate) border_type: BorderType,
    pub(crate) position: CellPosition,
}

impl BorderFragment {
    pub(crate) fn to_instance_key(&self) -> InstanceKey {
        InstanceKey::Position(self.position.row, self.position.col)
    }
}

#[derive(Default)]
pub(crate) struct BorderInstances {
    vector_instances: BTreeMap<BorderType, VectorInstances<String>>,
}

impl BorderInstances {
    pub(crate) fn add(
        &mut self,
        fragment: BorderFragment,
        instance: InstanceAttributes,
        device: &Device,
    ) {
        let instances = self
            .vector_instances
            .entry(fragment.border_type.clone())
            .or_insert_with(|| VectorInstances::new(fragment.border_type.to_key(), device));
        instances.insert(fragment.to_instance_key(), instance);
    }

    pub(crate) fn get_mut(&mut self, key: &BorderFragment) -> Option<&mut InstanceAttributes> {
        if let Some(instances) = self.vector_instances.get_mut(&key.border_type) {
            instances.get_mut(&key.to_instance_key())
        } else {
            None
        }
    }

    pub(crate) fn update(&mut self, device: &Device, queue: &Queue) {
        for instances in self.vector_instances.values_mut() {
            instances.update_buffer(device, queue)
        }
    }

    pub(crate) fn to_instances(&self) -> Vec<&VectorInstances<String>> {
        self.vector_instances.values().collect()
    }
}
