use skrifa::GlyphId;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FontRasterizerError {
    #[error("glyph not found. char: {0}")]
    GlyphNotFound(char),
    #[error("glyph index not found")]
    GlyphIndexNotFound,
    #[error("ensure buffer capacity failed. kind:{0:?}")]
    EnsureBufferCapacityFailed(BufferKind),
    #[error("outline glyph is failed. glyph_id:{0:?}")]
    NoOutlineGlyph(GlyphId),

    #[error("vector index not found")]
    VectorIndexNotFound,

    #[error("invalid svg format")]
    SvgParseError,

    #[error("path storage buffer is too large: {bytes} bytes exceeds {limit} bytes")]
    PathBufferTooLarge { bytes: u64, limit: u64 },
}

#[derive(Debug)]
pub enum BufferKind {
    Vertex,
    Index,
}
