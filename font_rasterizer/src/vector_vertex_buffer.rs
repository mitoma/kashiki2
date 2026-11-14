use std::{collections::BTreeMap, fmt::Debug};

use crate::{errors::FontRasterizerError, vector_vertex::VectorVertex};

struct BufferIndex {
    paths: wgpu::BindGroup,
    curve_count: u32,
}

pub struct VectorVertexBuffer<T> {
    buffer_index: BTreeMap<T, BufferIndex>,
}

impl<T> Default for VectorVertexBuffer<T>
where
    T: Ord + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> VectorVertexBuffer<T>
where
    T: Ord + Debug,
{
    pub(crate) fn new() -> Self {
        Self {
            buffer_index: BTreeMap::default(),
        }
    }

    pub fn has_key(&self, key: &T) -> bool {
        self.buffer_index.contains_key(key)
    }

    pub(crate) fn windfoil_draw_info(&self, key: &T) -> Option<(&wgpu::BindGroup, u32)> {
        let index = self.buffer_index.get(key)?;
        Some((&index.paths, index.curve_count))
    }

    pub fn append(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        key: T,
        glyph_data: VectorVertex,
    ) -> Result<(), FontRasterizerError> {
        let curves = glyph_data.curves();
        let paths = crate::windfoil::upload(device, &curves)?;
        self.buffer_index.insert(
            key,
            BufferIndex {
                paths,
                curve_count: curves.len() as u32,
            },
        );
        Ok(())
    }
}
