# Log

## [2026-11-06] fix | Card overlay surface の座標系を修正

- ModalWorld overlay に surface instances の経路を追加し、Tagbase meta card の面を RenderData batch に渡す
- TextEdit surface に glyph と同じ camera detail を設定し、`IGNORE_CAMERA` overlay での投影ずれを解消
- tagbase E2E golden を更新し、通常比較で全シナリオ成功

## [2026-11-05] refactor | Windfoil model batch を一括 resolve

- Windfoil に順序付き複数 Buffers の draw collection を追加し、normal/modal ごとの render pass と resolve を一回に集約
- Legacy は coverage/count のモデル間混在を避けるため per-batch composite fallback を維持
- command palette phase avg 約 20→1 ms、tagbase 約 6→<1 ms (debug、GPU timestamp ではない)
- E2E の微小な AA/量子化差を確認して PNG golden を更新し、通常比較で全シナリオ成功

## [2026-11-05] ingest | Model bound surface と E2E 描画検証を反映

- bound 面のモデル順合成、TextEdit の既定テーマ面、Legacy / Windfoil 共通合成段を設計・実装
- `model-bound-surface` decision と `model-surface-e2e` workflow、実装 source summary を追加
- `text-rendering-pipeline` と `index.md` を更新。tagbase memo の swap 中 snapshot を追加
- E2E golden を更新後、通常比較で全シナリオ成功。残課題は角丸・半透明・border 境界共有

## [2026-09-13] ingest | テキストラスタライズ品質改善を反映

- `font_rasterizer` の現行 WGSL を再確認し、signed coverage の格納 (`count.r/g/b`) と outline resolve の説明を更新
- `VectorVertexBuilder` のサブパス中心点探索と、`StraightenOutlineBuilder` / `straight_run_simplifier` によるほぼ直線の簡約を component / source / pipeline に反映
- debug 専用の MAX coverage 試行は historical な知見として扱い、現行 production の平均化ロジックと区別
- 残課題は conservative rasterization の条件差に対する signed coverage の品質評価と overlap remover の将来位置づけ

## [2026-08-14] ingest | Arborium への構文解析基盤移行を反映

- `Cargo.toml`、`highlighter/src/lib.rs`、`markdown_heading_splitter/src/lib.rs` を source として追加
- Arborium の language feature、Markdown inline / code fence の二段解析、見出し抽出経路を component に整理
- `index.md` に syntax analysis の component / source を追加

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

## [2026-07-19] ingest | Tier 3 の構造拡張を反映

- shader art system の component / source を追加
- overlap remover の component / decision / source を追加
- `ModalWorld` と代表 world 実装を整理した world-scene-architecture concept と source を追加
- `index.md` の Tier 3 backlog を完了扱いに更新

## [2026-07-19] ingest | 外部 URL source の運用を追加

- raw を置かない URL source 取り込みのルールを `schema.md` と workflow に追加
- Evan Wallace の `Easy Scalable Text Rendering on the GPU` を source summary として追加
- text rendering pipeline に外部由来の基礎アルゴリズム参照を追加

## [2026-07-19] ingest | analytical anti-aliasing の外部記事を追加

- FrostKiwi の `AAA - Analytical Anti-Aliasing` を raw なし URL source として追加
- anti-aliasing strategy と既存 AA source summary を更新

## [2026-11-01] query | E2E 高速化の調査

- debug / release / `WGPU_DEBUG=1` の各構成で `tagbase_memo` シナリオの実行時間を計測
- release が遅いのは、DX12 FXC が最適化ありで windfoil シェーダーをコンパイルするためと特定
- `decisions/e2e-performance.md` を draft（保留）として追加し、`index.md` に追記
- 残課題: 対応する場合は release + `WGPU_DEBUG=1` を第一候補として再検討する

## ルール

- 見出しは `## [YYYY-MM-DD] kind | title` 形式にする
- `kind` は `ingest`, `query`, `lint`, `bootstrap`, `refactor` を想定する
- 1 エントリには「何を読んだか」「どのページを更新したか」「残課題は何か」を短く残す