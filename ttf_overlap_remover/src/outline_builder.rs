use skrifa::outline::OutlinePen;
use tiny_skia_path::{Path, PathBuilder};

use crate::remove_path_overlap;

/// オーバーラップ除去機能付き OutlinePen。
///
/// skrifa の `OutlineGlyph::draw()` に渡して使用する。
/// グリフのアウトラインを内部に蓄積し、`removed_paths()` で重複除去後のパスを取得できる。
#[derive(Debug)]
pub struct OverlapRemoveOutlineBuilder {
    builder: Option<PathBuilder>,
    paths: Vec<Path>,
}

impl Default for OverlapRemoveOutlineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlapRemoveOutlineBuilder {
    fn new() -> Self {
        Self {
            builder: Some(PathBuilder::new()),
            paths: Vec::new(),
        }
    }

    /// 元のパス（重複除去前）を取得する
    pub fn paths(&self) -> Vec<Path> {
        self.paths.clone()
    }

    /// 重複除去後のパスを取得する
    pub fn removed_paths(&self) -> Vec<Path> {
        remove_path_overlap(self.paths.clone())
    }

    /// 重複除去後のパスを OutlinePen にフィードバックする
    pub fn outline<T>(&self, builder: &mut T)
    where
        T: OutlinePen,
    {
        let removed = self.removed_paths();
        for path in &removed {
            let mut first = true;
            for segment in path.segments() {
                match segment {
                    tiny_skia_path::PathSegment::MoveTo(p) => {
                        builder.move_to(p.x, p.y);
                        first = false;
                    }
                    tiny_skia_path::PathSegment::LineTo(p) => {
                        if first {
                            builder.move_to(p.x, p.y);
                            first = false;
                        }
                        builder.line_to(p.x, p.y);
                    }
                    tiny_skia_path::PathSegment::QuadTo(c, p) => {
                        builder.quad_to(c.x, c.y, p.x, p.y);
                    }
                    tiny_skia_path::PathSegment::CubicTo(c1, c2, p) => {
                        builder.curve_to(c1.x, c1.y, c2.x, c2.y, p.x, p.y);
                    }
                    tiny_skia_path::PathSegment::Close => {
                        builder.close();
                    }
                }
            }
        }
    }
}

impl OutlinePen for OverlapRemoveOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.as_mut().unwrap().move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.as_mut().unwrap().line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.builder.as_mut().unwrap().quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.builder
            .as_mut()
            .unwrap()
            .cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        let mut builder = self.builder.replace(PathBuilder::new()).unwrap();
        builder.close();
        if let Some(path) = builder.finish() {
            self.paths.push(path);
        }
    }
}
