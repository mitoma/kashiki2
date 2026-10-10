use std::{collections::BTreeMap, fmt::Debug};

use crate::{errors::FontRasterizerError, vector_vertex::VectorVertex, windfoil};

#[derive(Debug)]
pub(crate) struct DrawInfo<'a> {
    pub(crate) windfoil: &'a wgpu::BindGroup,
}

pub struct VectorVertexBuffer<T> {
    paths: BTreeMap<T, wgpu::BindGroup>,
}

impl<T: Ord + Debug> Default for VectorVertexBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + Debug> VectorVertexBuffer<T> {
    pub(crate) fn new() -> Self {
        Self {
            paths: BTreeMap::new(),
        }
    }

    pub(crate) fn draw_info(&self, key: &T) -> Result<DrawInfo<'_>, FontRasterizerError> {
        let windfoil = self
            .paths
            .get(key)
            .ok_or(FontRasterizerError::VectorIndexNotFound)?;
        Ok(DrawInfo { windfoil })
    }

    pub fn has_key(&self, key: &T) -> bool {
        self.paths.contains_key(key)
    }

    pub fn append(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        key: T,
        glyph_data: VectorVertex,
    ) -> Result<(), FontRasterizerError> {
        self.paths
            .insert(key, windfoil::upload(device, &glyph_data)?);
        Ok(())
    }
}
