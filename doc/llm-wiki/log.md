# Log

## [2026-07-19] bootstrap | 初期 schema と seed pages を作成

- `doc/llm-wiki/` を新設
- `README.md`, `schema.md`, `index.md`, `log.md` を追加
- 初期 seed として concepts / decisions / workflows / sources を追加
- Tier 1 と Tier 2 backlog を `index.md` に記録

## [2026-07-19] ingest | Tier 1 の source を反映

- `doc/memo/font.md` を source 化し、glyph モデルの component を追加
- `/memories/repo/text_buffer_design.md` を source 化し、layout / preedit の説明へ反映
- `/memories/repo/vector_vertex_aa_analysis.md` を historical source 化
- `/memories/repo/font_overlap_artifact_notes.md` を source 化し、AA strategy の保留論点を更新
- `index.md` の Tier 1 backlog を完了扱いに更新

## [2026-07-19] ingest | Tier 2 第一波のコード source を反映

- `font_rasterizer/src/rasterizer_pipeline.rs` を source 化し、rendering pipeline の実行段を明文化
- `text_buffer/src/layout.rs`, `editor.rs`, `action.rs` を source 化
- `editor-buffer-model` と `action-system` の概念ページを追加
- `index.md` の Tier 2 backlog を部分更新し、残件を vector vertex と shader 群に絞った

## [2026-07-19] ingest | Tier 2 第二波のコード source を反映

- `font_rasterizer/src/vector_vertex.rs` を source 化し、vector vertex builder component を追加
- `font_rasterizer/src/shader/overlap_shader.wgsl`, `outline_shader.wgsl` を source 化
- text rendering pipeline と anti-aliasing strategy を更新
- `index.md` の Tier 2 backlog を完了扱いに更新

## ルール

- 見出しは `## [YYYY-MM-DD] kind | title` 形式にする
- `kind` は `ingest`, `query`, `lint`, `bootstrap`, `refactor` を想定する
- 1 エントリには「何を読んだか」「どのページを更新したか」「残課題は何か」を短く残す