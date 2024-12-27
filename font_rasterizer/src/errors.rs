use rustybuzz::ttf_parser::GlyphId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FontRasterizerError {
    #[error("glyph not found. char: {0}")]
    GlyphNotFound(char),
    #[error("glyph index not found")]
    GlyphIndexNotFound,
    // バッファの確保に失敗しているエラー
    #[error("Failed to allocate buffer. kind: {0:?}")]
    BufferAllocationFailed(BufferKind),
    #[error("outline glyph is failed. glyph_id:{0:?}")]
    NoOutlineGlyph(GlyphId),
}

#[derive(Debug)]
pub enum BufferKind {
    Vertex,
    Index,
}
