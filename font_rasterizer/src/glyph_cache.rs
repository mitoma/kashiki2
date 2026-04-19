use std::path::PathBuf;

use font_collector::FontData;
use redb::{Database, TableDefinition};

use crate::{
    char_width_calcurator::CharWidth,
    font_converter::GlyphVertex,
    vector_vertex::{VectorVertex, Vertex},
};

const GLYPH_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("glyphs");

/// フォントバイナリの FNV-1a ハッシュを計算する（実行間で安定した値を返す）
fn fonts_hash(fonts: &[FontData]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET: u64 = 14695981039346656037;
    let mut hash = FNV_OFFSET;
    for font in fonts {
        for &byte in &font.binary {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

fn cache_db_path(fonts: &[FontData]) -> PathBuf {
    let hash = fonts_hash(fonts);
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kashikishi")
        .join(format!("glyph_cache_{hash:016x}.redb"))
}

pub(crate) struct GlyphCache {
    db: Database,
}

impl GlyphCache {
    /// キャッシュを開く。失敗した場合は None を返しログに警告を出す。
    pub(crate) fn open(fonts: &[FontData]) -> Option<Self> {
        let path = cache_db_path(fonts);
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            log::warn!("グリフキャッシュディレクトリの作成に失敗: {e}");
            return None;
        }
        match Database::create(&path) {
            Ok(db) => {
                log::info!("グリフキャッシュを開きました: {}", path.display());
                Some(Self { db })
            }
            Err(e) => {
                log::warn!("グリフキャッシュのオープンに失敗: {e}");
                None
            }
        }
    }

    fn make_key(c: char, width: CharWidth) -> [u8; 5] {
        let c_bytes = (c as u32).to_le_bytes();
        let w_byte = match width {
            CharWidth::Regular => 0u8,
            CharWidth::Wide => 1u8,
        };
        [c_bytes[0], c_bytes[1], c_bytes[2], c_bytes[3], w_byte]
    }

    /// キャッシュからグリフ頂点データを取得する。存在しない場合は None を返す。
    pub(crate) fn get(&self, c: char, width: CharWidth) -> Option<GlyphVertex> {
        let key = Self::make_key(c, width);
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(GLYPH_TABLE).ok()?;
        let guard = table.get(key.as_slice()).ok()??;
        deserialize_glyph_vertex(guard.value())
    }

    /// グリフ頂点データをキャッシュに保存する。失敗した場合はログに警告を出す。
    pub(crate) fn set(&self, glyph: &GlyphVertex, width: CharWidth) {
        let key = Self::make_key(glyph.c, width);
        let value = serialize_glyph_vertex(glyph);
        if let Err(e) = self.set_inner(&key, &value) {
            log::warn!("グリフキャッシュへの書き込みに失敗: {e}");
        }
    }

    fn set_inner(
        &self,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(GLYPH_TABLE)?;
            table.insert(key, value)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

// ---- シリアライズ / デシリアライズ ----

fn serialize_vector_vertex(v: &VectorVertex, buf: &mut Vec<u8>) {
    // Vertex は bytemuck::Pod なのでそのままバイト列に変換できる
    let vertex_bytes: &[u8] = bytemuck::cast_slice(&v.vertex);
    buf.extend_from_slice(&(v.vertex.len() as u32).to_le_bytes());
    buf.extend_from_slice(vertex_bytes);
    buf.extend_from_slice(&(v.index.len() as u32).to_le_bytes());
    for &i in &v.index {
        buf.extend_from_slice(&i.to_le_bytes());
    }
}

fn deserialize_vector_vertex(data: &[u8], pos: &mut usize) -> Option<VectorVertex> {
    let vertex_len = u32::from_le_bytes(data.get(*pos..*pos + 4)?.try_into().ok()?) as usize;
    *pos += 4;
    // bytemuck::cast_slice はアライメントを要求するため、フィールドを個別に読み出す
    let mut vertex = Vec::with_capacity(vertex_len);
    for _ in 0..vertex_len {
        let x = f32::from_le_bytes(data.get(*pos..*pos + 4)?.try_into().ok()?);
        *pos += 4;
        let y = f32::from_le_bytes(data.get(*pos..*pos + 4)?.try_into().ok()?);
        *pos += 4;
        let vertex_type = u32::from_le_bytes(data.get(*pos..*pos + 4)?.try_into().ok()?);
        *pos += 4;
        vertex.push(Vertex {
            position: [x, y],
            vertex_type,
        });
    }

    let index_len = u32::from_le_bytes(data.get(*pos..*pos + 4)?.try_into().ok()?) as usize;
    *pos += 4;
    let mut index = Vec::with_capacity(index_len);
    for _ in 0..index_len {
        index.push(u32::from_le_bytes(
            data.get(*pos..*pos + 4)?.try_into().ok()?,
        ));
        *pos += 4;
    }
    Some(VectorVertex { vertex, index })
}

fn serialize_glyph_vertex(g: &GlyphVertex) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(g.c as u32).to_le_bytes());
    serialize_vector_vertex(&g.h_vertex, &mut buf);
    match &g.v_vertex {
        Some(v) => {
            buf.push(1);
            serialize_vector_vertex(v, &mut buf);
        }
        None => buf.push(0),
    }
    buf
}

fn deserialize_glyph_vertex(data: &[u8]) -> Option<GlyphVertex> {
    let mut pos = 0;
    let c = char::from_u32(u32::from_le_bytes(data.get(pos..pos + 4)?.try_into().ok()?))?;
    pos += 4;
    let h_vertex = deserialize_vector_vertex(data, &mut pos)?;
    let v_vertex = if *data.get(pos)? == 1 {
        pos += 1;
        Some(deserialize_vector_vertex(data, &mut pos)?)
    } else {
        None
    };
    Some(GlyphVertex {
        c,
        h_vertex,
        v_vertex,
    })
}
