## Non-Zero Winding Rule 実装プラン（`@builtin(front_facing)` による GPU 側符号判定）

### 概要

現在の Even-Odd 法を Non-Zero Winding Rule に置き換え、`ttf_overlap_remover` を不要にする。

当初は CPU 側でサブパスの巻き方向を計算し頂点データに埋め込むアプローチ（アプローチ B）を計画したが、centroid-fan 三角形分割では凹多角形の重なり部分で同一サブパス内の三角形が逆符号を持つ必要があるため、サブパス単位の一律符号では正しく動作しなかった。

最終的に、フラグメントシェーダーの `@builtin(front_facing)` で三角形のスクリーン空間での向きから符号を決定するアプローチを採用した。CPU 側の頂点データ変更は不要。

---


### Phase 1: カウントテクスチャのフォーマット変更 ✅

**対象ファイル:** `font_rasterizer/src/rasterizer_renderrer.rs`

#### 1-1. テクスチャフォーマットを符号付きに変更

```rust
// 変更前
wgpu::TextureFormat::Bgra8Unorm
// 変更後
wgpu::TextureFormat::Rgba16Float
```

`Rgba16Float` を使う理由:
- 負の値を格納できる
- 十分な精度がある（8bit Snorm だと 127 段階しかなく、複雑なグリフで飽和する可能性）
- ブレンド（Add）が動作する

#### 1-2. ブレンドステートは変更不要

現在の `One + One (Add)` のまま正しく動作する（正の値と負の値が加算される）。

#### 1-3. `cull_mode: None` が必要

`front_facing` による符号判定を正しく機能させるため、`cull_mode` は `None` でなければならない（背面カリングすると `-1` 側の三角形が描画されない）。既にそうなっていたため変更不要だった。

---


### Phase 2: Overlap シェーダーの変更 ✅

**対象ファイル:** `font_rasterizer/src/shader/overlap_shader.wgsl`, `overlap_shader.debug.wgsl`

#### 2-1. `fs_main` で `@builtin(front_facing)` による符号付き加算

頂点データへの `winding_sign` 追加は不要。代わりに `fs_main` の引数に `@builtin(front_facing)` を追加し、三角形のスクリーン空間での向きで符号を決定する。

```wgsl
@fragment
fn fs_main(@builtin(front_facing) front_facing: bool, in: VertexOutput) -> FragmentOutput {
    // Non-Zero Winding Rule: 三角形のスクリーン空間での向きで符号を決定する
    let winding_sign = select(-1.0, 1.0, front_facing);
    // ...
    output.count.r = UNIT * winding_sign;
    // ...
}
```

これにより:
- 外側パス（CW/CCW）の三角形は一方向を向き `+UNIT` を出力
- 穴パス（逆巻き）の三角形は逆方向を向き `-UNIT` を出力
- **凹多角形の重なり部分**: centroid-fan で折り返す三角形は自然に逆向きになり、重なったエリアのカウントが相殺される

**なぜ頂点データへの埋め込み（アプローチ B）が不適切だったか:**

centroid-fan 三角形分割では、凹多角形の場合に一部の fan 三角形がスクリーン空間で「裏返し」になる。この裏返し三角形は、凹部の「はみ出し」を打ち消すために逆符号で加算される必要がある。サブパス単位で一律に `+1` や `-1` を割り当てると、この打ち消しが機能しない。`front_facing` は各三角形のスクリーン空間での向きを自動的に判定するため、正しく動作する。

---


### Phase 3: Outline シェーダーの変更 ✅

**対象ファイル:** `font_rasterizer/src/shader/outline_shader.wgsl`, `outline_shader.debug.wgsl`

#### 3-1. Even-Odd 判定を Non-Zero 判定に置き換え

```wgsl
let winding = overlap_count.r;
let is_inside = abs(winding) > WINDING_THRESHOLD;  // Non-Zero 判定
```

#### 3-2. アンチエイリアス処理の再設計

Non-Zero では符号付き値の `abs()` で判定する。加えて、重なり領域の内部エッジで偽の AA アーティファクトが出る問題への対処として、winding 数が単一層境界付近のときだけ AA を適用する条件を追加:

```wgsl
if is_inside {
    if abs(alpha_counts) > WINDING_THRESHOLD && abs(winding) <= UNIT + HARFUNIT {
        // 単一層の境界付近: アルファで滑らかに
        let alpha = clamp(abs(alpha_accum) * ALPHA_STEP / abs(alpha_counts * 256.0), 0.0, 1.0);
        return vec4<f32>(color.rgb, alpha);
    } else {
        // 重なり深部または非エッジ: 完全不透明
        return vec4<f32>(color.rgb, 1.0);
    }
} else {
    if abs(alpha_counts) > WINDING_THRESHOLD {
        let alpha = 1.0 - clamp(abs(alpha_accum) * ALPHA_STEP / abs(alpha_counts * 256.0), 0.0, 1.0);
        if alpha > 0.001 {
            return vec4<f32>(color.rgb, alpha);
        }
    }
    return vec4<f32>(color.rgb, 0.0);
}
```

**重なり領域 AA アーティファクトの原因と対策:**

加算ブレンドされた count テクスチャでは、サブパスAのエッジ上にサブパスBが重なると `(R=2*UNIT, G=alpha/16, B=UNIT)` のような値になる。`alpha_counts > 0` だけで AA を適用すると、本来不透明であるべき重なり深部まで半透明になる。`abs(winding) <= UNIT + HARFUNIT` 条件を加えることで、winding が単一層境界（≈ ±UNIT）のときだけ AA を適用し、重なり深部（winding ≥ 2*UNIT）では完全不透明を返す。

---


### Phase 4: `ttf_overlap_remover` の除去（未実施）

#### 4-1. `font_converter.rs` の変更

```rust
// 変更前
let rect = if remove_overlap {
    let mut overlap_builder = OverlapRemoveOutlineBuilder::default();
    let rect = face.outline_glyph(glyph_id, &mut overlap_builder)...;
    overlap_builder.outline(&mut builder);
    rect
} else {
    face.outline_glyph(glyph_id, &mut builder)...
};

// 変更後: 常に直接 builder に渡す
let rect = face.outline_glyph(glyph_id, &mut builder)
    .ok_or(FontRasterizerError::NoOutlineGlyph(glyph_id))?;
```

#### 4-2. 依存関係の削除

- `font_rasterizer/Cargo.toml` から `ttf_overlap_remover` の依存を削除
- `font_converter.rs` から `use ttf_overlap_remover::OverlapRemoveOutlineBuilder;` を削除
- `is_remove_outline_fontname()` メソッドを削除
- `build()` メソッドの `remove_overlap` パラメータを削除

#### 4-3. `ttf_overlap_remover` クレートの処遇

テストコードとして残すか、完全に削除するかは判断が必要。Non-Zero への移行検証が完了するまでは残しておき、最終的に削除するのが安全。

---


### Phase 5: テストと検証（部分的に実施）

#### 5-1. ビジュアルテスト

`ui_support/examples/aa_test.rs` で「あ」の描画を確認済み。以下の追加検証が望ましい:

- 通常の日本語文字（漢字、ひらがな）
- 重なりのある絵文字（🐢、🐖、🎍 等）
- ASCII 文字
- SVG ベクター画像（`vector_vertex_buffer` 経由）

#### 5-2. パフォーマンス計測

- `ttf_overlap_remover` 削除によるグリフ変換時間の短縮を計測
- テクスチャフォーマット変更（8bit → 16bit float）による VRAM 使用量増加の影響を確認

---


### 実装順序（実際に行った手順）

1. **Phase 1**（テクスチャフォーマット変更）→ **Phase 2**（overlap shader の `front_facing` 対応）→ **Phase 3**（outline shader の Non-Zero 判定 + AA 調整）を実装
2. `aa_test` でレンダリング結果を検証
3. 重なり領域の AA アーティファクトを発見し、outline shader に winding 境界条件を追加
4. **Phase 4**（`ttf_overlap_remover` 除去）は未実施

---


### 変更ファイル一覧

| ファイル | 変更種別 | 状態 |
| --- | --- | --- |
| `font_rasterizer/src/rasterizer_renderrer.rs` | 小（テクスチャフォーマット変更） | ✅ |
| `font_rasterizer/src/shader/overlap_shader.wgsl` | 小（`fs_main` に `front_facing` 追加、符号付き出力） | ✅ |
| `font_rasterizer/src/shader/overlap_shader.debug.wgsl` | 同上 | ✅ |
| `font_rasterizer/src/shader/outline_shader.wgsl` | 中（Non-Zero 判定 + AA 境界条件） | ✅ |
| `font_rasterizer/src/shader/outline_shader.debug.wgsl` | 同上 | ✅ |
| `font_rasterizer/src/font_converter.rs` | 中（overlap remover 除去） | 未実施 |
| `font_rasterizer/Cargo.toml` | 小（依存削除） | 未実施 |

---


### 注意点・リスク

1. **SVG ベクター画像:** `svg.rs` 経由で `VectorVertexBuilder` を使っている場合も `front_facing` による符号判定は自動的に機能する。ただし、Even-Odd 前提の SVG があると表示が崩れる可能性がある。

2. **テクスチャフォーマットの互換性:** `Rgba16Float` は WebGPU でも広くサポートされているが、VRAM 使用量が倍になる。問題があれば `Rgba8Snorm` を検討（ただし精度が下がる）。

3. **`debug.wgsl` ファイル:** 本番用とデバッグ用の 2 つのシェーダーファイルがあるので、両方を同じように更新する必要がある。

4. **`cull_mode`:** `front_facing` アプローチは `cull_mode: None` が前提。背面カリングを有効にすると壊れる。
