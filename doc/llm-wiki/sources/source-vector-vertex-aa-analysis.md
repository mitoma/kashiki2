---
title: Source Summary - Vector Vertex AA Analysis
kind: source
status: historical
updated: 2026-07-19
source_refs:
  - ../../../memories/repo/vector_vertex_aa_analysis.md
related_pages:
  - ../decisions/anti-aliasing-strategy.md
  - ../concepts/text-rendering-pipeline.md
---

# Source Summary - Vector Vertex AA Analysis

## 対象 source

- `/memories/repo/vector_vertex_aa_analysis.md`

## 要約

- vector vertex と shader の相互作用を調べた結果、internal fan edge にも AA が適用される構造がアーティファクト原因として整理されている
- 凸性タグ付けと fill coverage による救済案は、輪郭間の巻き相殺を尊重できず棄却された
- ベースラインとしては clean な `fwidth` ベース維持が推奨され、signed coverage accumulation が次の正攻法候補として残っている

## historical に重要な点

- 失敗案を削除せず残すことで、なぜ採用しなかったかを再学習できる
- `vertex_type`、`triangle_type`、`wait` の関係は今後の code source 化の前提知識になる