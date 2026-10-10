use std::path::PathBuf;

use font_collector::FontData;
use redb::{Database, ReadableDatabase, TableDefinition, TypeName, Value};

use crate::{
    char_width_calcurator::CharWidth,
    font_converter::GlyphVertex,
    vector_vertex::{QuadraticCurve, VectorVertex},
};

const GLYPH_TABLE: TableDefinition<&str, GlyphVertex> = TableDefinition::new("glyphs");
const CACHE_FORMAT_VERSION: u32 = 2;

impl Value for GlyphVertex {
    type SelfType<'a>
        = GlyphVertex
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        deserialize_glyph_vertex(data).expect("deserialize glyph vertex from bytes")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        serialize_glyph_vertex(value)
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("GlyphCurvesV2")
    }
}

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

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kashikishi")
}

fn cache_db_path(fonts: &[FontData]) -> PathBuf {
    let hash = fonts_hash(fonts);
    cache_dir().join(format!(
        "glyph_cache_v{CACHE_FORMAT_VERSION}_{hash:016x}.redb"
    ))
}

/// グリフキャッシュ（`glyph_cache_*.redb`）をすべて削除する。
/// フォントの組み合わせごとに別ファイルが存在するため、全ファイルを対象とする。
pub fn clear_glyph_cache() {
    let dir = cache_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!(
                "グリフキャッシュディレクトリが存在しません: {}",
                dir.display()
            );
            return;
        }
        Err(e) => {
            log::warn!("グリフキャッシュディレクトリの読み込みに失敗: {e}");
            return;
        }
    };

    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let is_cache_file = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("glyph_cache_") && name.ends_with(".redb"));
        if is_cache_file {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    log::info!("グリフキャッシュを削除しました: {}", path.display());
                    removed += 1;
                }
                Err(e) => log::warn!("グリフキャッシュの削除に失敗 ({}): {e}", path.display()),
            }
        }
    }
    log::info!("グリフキャッシュを {removed} 件削除しました");
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

    fn make_key(c: char, width: CharWidth) -> String {
        let width_key = match width {
            CharWidth::Regular => "R".to_string(),
            CharWidth::Wide => "W".to_string(),
            CharWidth::Proportional(width) => format!("P{:08x}", width.to_bits()),
        };
        format!("{}:{width_key}", c as u32)
    }

    /// キャッシュからグリフ頂点データを取得する。存在しない場合は None を返す。
    pub(crate) fn get(&self, c: char, width: CharWidth) -> Option<GlyphVertex> {
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(GLYPH_TABLE).ok()?;
        table
            .get(Self::make_key(c, width).as_str())
            .ok()
            .flatten()
            .map(|entry| entry.value())
    }

    /// グリフ頂点データをキャッシュに保存する。失敗した場合はログに警告を出す。
    pub(crate) fn set(&self, glyph: &GlyphVertex, width: CharWidth) {
        if let Err(e) = self.set_inner(Self::make_key(glyph.c, width).as_str(), glyph) {
            log::warn!("グリフキャッシュへの書き込みに失敗: {e}");
        }
    }

    fn set_inner(
        &self,
        key: &str,
        glyph: &GlyphVertex,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(GLYPH_TABLE)?;
            table.insert(key, glyph)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

// ---- シリアライズ / デシリアライズ ----

fn serialize_vector_vertex(v: &VectorVertex, buf: &mut Vec<u8>) {
    buf.extend_from_slice(&(v.curves().len() as u32).to_le_bytes());
    for curve in v.curves() {
        for point in curve.points() {
            for coordinate in point {
                buf.extend_from_slice(&coordinate.to_le_bytes());
            }
        }
    }
}

fn deserialize_vector_vertex(data: &[u8], pos: &mut usize) -> Option<VectorVertex> {
    let curve_count =
        u32::from_le_bytes(data.get(*pos..pos.checked_add(4)?)?.try_into().ok()?) as usize;
    *pos += 4;
    let byte_count = curve_count.checked_mul(std::mem::size_of::<QuadraticCurve>())?;
    let end = pos.checked_add(byte_count)?;
    let bytes = data.get(*pos..end)?;
    *pos = end;
    let curves = bytes
        .as_chunks::<24>()
        .0
        .iter()
        .map(|bytes| {
            let coordinate =
                |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            QuadraticCurve {
                start: [coordinate(0), coordinate(4)],
                control: [coordinate(8), coordinate(12)],
                end: [coordinate(16), coordinate(20)],
            }
        })
        .collect();
    Some(VectorVertex { curves })
}

fn serialize_glyph_vertex(g: &GlyphVertex) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&CACHE_FORMAT_VERSION.to_le_bytes());
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
    let version = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
    if version != CACHE_FORMAT_VERSION {
        return None;
    }
    let mut pos = 4;
    let c = char::from_u32(u32::from_le_bytes(data.get(pos..pos + 4)?.try_into().ok()?))?;
    pos += 4;
    let h_vertex = deserialize_vector_vertex(data, &mut pos)?;
    let tag = *data.get(pos)?;
    pos += 1;
    let v_vertex = match tag {
        1 => Some(deserialize_vector_vertex(data, &mut pos)?),
        0 => None,
        _ => return None,
    };
    if pos != data.len() {
        return None;
    }
    Some(GlyphVertex {
        c,
        h_vertex,
        v_vertex,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VectorVertexBuilder;

    #[test]
    fn directed_curves_round_trip_with_vertical_variant() {
        let mut builder = VectorVertexBuilder::new();
        builder.move_to(1.0, 2.0);
        builder.line_to(3.0, 2.0);
        builder.quad_to(4.0, 3.0, 3.0, 4.0);
        builder.close();
        let glyph = GlyphVertex {
            c: 'あ',
            h_vertex: builder.build(),
            v_vertex: Some(VectorVertex {
                curves: vec![QuadraticCurve {
                    start: [-1.0, 2.0],
                    control: [0.0, 4.0],
                    end: [1.0, 2.0],
                }],
            }),
        };
        let bytes = serialize_glyph_vertex(&glyph);
        let decoded = deserialize_glyph_vertex(&bytes).unwrap();
        assert_eq!(decoded.c, glyph.c);
        assert_eq!(decoded.h_vertex.curves(), glyph.h_vertex.curves());
        assert_eq!(
            decoded.v_vertex.unwrap().curves(),
            glyph.v_vertex.unwrap().curves()
        );
        assert!(
            cache_db_path(&[])
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("glyph_cache_v2_")
        );
    }

    #[test]
    fn empty_glyph_round_trips_without_vertical_variant() {
        let glyph = GlyphVertex {
            c: ' ',
            h_vertex: VectorVertexBuilder::new().build(),
            v_vertex: None,
        };
        let decoded = deserialize_glyph_vertex(&serialize_glyph_vertex(&glyph)).unwrap();
        assert_eq!(decoded.c, ' ');
        assert!(decoded.h_vertex.curves().is_empty());
        assert!(decoded.v_vertex.is_none());
    }

    #[test]
    fn rejects_legacy_truncated_and_invalid_payloads() {
        let glyph = GlyphVertex {
            c: 'A',
            h_vertex: VectorVertexBuilder::new().build(),
            v_vertex: None,
        };
        let bytes = serialize_glyph_vertex(&glyph);
        for length in 0..bytes.len() {
            assert!(deserialize_glyph_vertex(&bytes[..length]).is_none());
        }
        let mut legacy = bytes.clone();
        legacy[..4].copy_from_slice(&1u32.to_le_bytes());
        assert!(deserialize_glyph_vertex(&legacy).is_none());
        let mut invalid_count = bytes.clone();
        invalid_count[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(deserialize_glyph_vertex(&invalid_count).is_none());
        let mut invalid_tag = bytes.clone();
        *invalid_tag.last_mut().unwrap() = 2;
        assert!(deserialize_glyph_vertex(&invalid_tag).is_none());
        let mut trailing = bytes;
        trailing.push(0);
        assert!(deserialize_glyph_vertex(&trailing).is_none());
    }
}
