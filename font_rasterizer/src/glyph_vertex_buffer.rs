use std::{collections::HashSet, fmt::Debug, sync::Arc};

use font_collector::FontData;
use log::debug;
use text_buffer::editor::CharWidthResolver;

use crate::{
    char_width_calcurator::{CharWidth, CharWidthCalculator},
    errors::FontRasterizerError,
    font_converter::{FontVertexConverter, GlyphVertex},
    vector_vertex_buffer::{DrawInfo, VectorVertexBuffer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Direction {
    #[default]
    Horizontal,
    Vertical,
}

impl Direction {
    pub fn toggle(&self) -> Direction {
        match self {
            Direction::Horizontal => Direction::Vertical,
            Direction::Vertical => Direction::Horizontal,
        }
    }
}

pub struct GlyphVertexBuffer {
    font_vertex_converter: FontVertexConverter,
    char_width_calculator: Arc<CharWidthCalculator>,

    registerd_chars: HashSet<char>,
    vector_vertex_buffer: VectorVertexBuffer<(char, Direction)>,
}

impl GlyphVertexBuffer {
    pub fn new(
        font_binaries: Arc<Vec<FontData>>,
        char_width_calculator: Arc<CharWidthCalculator>,
    ) -> GlyphVertexBuffer {
        let font_vertex_converter = FontVertexConverter::new(font_binaries);
        GlyphVertexBuffer {
            font_vertex_converter,
            char_width_calculator,
            registerd_chars: HashSet::new(),
            vector_vertex_buffer: VectorVertexBuffer::new(),
        }
    }

    pub(crate) fn draw_info(
        &'_ self,
        c: &char,
        direction: &Direction,
    ) -> Result<DrawInfo<'_>, FontRasterizerError> {
        if direction == &Direction::Vertical
            && let Ok(info) = self
                .vector_vertex_buffer
                .draw_info(&(*c, Direction::Vertical))
        {
            return Ok(info);
        }
        self.vector_vertex_buffer
            .draw_info(&(*c, Direction::Horizontal))
    }

    pub fn registerd_chars(&self) -> HashSet<char> {
        self.registerd_chars.clone()
    }

    pub fn append_chars(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chars: HashSet<char>,
    ) -> Result<(), FontRasterizerError> {
        // 既にバッファに登録済みの char は除外する。
        let chars = chars
            .into_iter()
            .filter(|c| !self.registerd_chars.contains(c))
            .collect::<HashSet<_>>();
        if chars.is_empty() {
            return Ok(());
        }

        debug!("registerd_chars:{:?}", self.registerd_chars);
        debug!("chars:{:?}", chars);

        // char を全て Glyph 情報に変換する
        let mut glyphs = chars
            .iter()
            .flat_map(|c| {
                let width = self.char_width_calculator.get_width(*c);
                self.font_vertex_converter.convert(*c, width)
            })
            .collect::<Vec<_>>();

        while let Some(GlyphVertex {
            c,
            h_vertex,
            v_vertex,
        }) = glyphs.pop()
        {
            self.vector_vertex_buffer.append(
                device,
                queue,
                (c, Direction::Horizontal),
                h_vertex,
            )?;
            v_vertex.and_then(|v_vertex| {
                self.vector_vertex_buffer
                    .append(device, queue, (c, Direction::Vertical), v_vertex)
                    .ok()
            });
            self.registerd_chars.insert(c);
        }
        Ok(())
    }

    pub fn width(&self, c: char) -> CharWidth {
        self.char_width_calculator.get_width(c)
    }
}

impl CharWidthResolver for GlyphVertexBuffer {
    fn resolve_width(&self, c: char) -> usize {
        match self.width(c) {
            CharWidth::Regular => 1,
            CharWidth::Wide => 2,
        }
    }
}
