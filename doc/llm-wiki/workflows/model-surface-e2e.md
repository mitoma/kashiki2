---
title: Model Surface E2E Workflow
kind: workflow
status: production
updated: 2026-11-05
source_refs:
  - ../../ai-agent/plans/model-surface-background.md
  - ../../../kashikishi/tests/e2e_scenarios.rs
  - ../../../kashikishi/e2e/scenarios/tagbase_memo.jsonl
related_pages:
  - ../decisions/model-bound-surface.md
  - ../concepts/text-rendering-pipeline.md
---

# Model Surface E2E Workflow

## シナリオ作成

- 既存操作でモデルの追加、重なり、位置や bound の遷移を起こし、`action_recorder:snapshot` を状態ごとに置く。
- アニメーション途中の検証では、操作直後に短い `wait` を置いて中間 snapshot を取り、その後に完了状態も記録する。
- 文字 snapshot はモデルツリーなどの論理回帰、PNG snapshot は面の遮蔽・境界・描画順の回帰として使い分ける。

## 実行・確認

1. 個別シナリオを `cargo run -p kashikishi -- test --scenario <path> --snapshot-dir target/e2e/<name>` で再生し、PNG の位置と重なりを目視する。
2. 期待値を更新する場合は `UPDATE_SNAPSHOTS=1 mise r e2e` を実行する。PNG 差分を確認してから受け入れる。
3. 更新後は環境変数を解除し、`mise r e2e` を再実行して通常の golden 比較を行う。
4. 変更全体の必須ゲートとして `mise r check` を実行する。

PNG 比較には許容誤差があるため、成功だけで幾何形状が正しいとは判断しない。必要に応じて一時的に識別しやすい RGB 色を使って surface geometry と遮蔽を切り分け、確認後はテーマ色へ戻して golden を更新する。