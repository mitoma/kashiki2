# "E" 中央線ジオメトリ構造の詳細分析

## 概要

本文書は、フォント文字 "E" の中央線（横直線）がどのようにジオメトリとして生成・処理されるか、特に直線（is_line）の待機値（wait.x, wait.y, wait.z）がどのように設定・計算されるかを詳細に解説します。

## 処理フロー全体

```
TTF フォント → 解析 → OutlineBuilder → VectorVertexBuilder → 頂点&インデックス
     ↓
   move_to, line_to 呼び出し
     ↓
   wait: FlipFlop enum
     ↓
   Vertex Shader (vertex_type → wait, triangle_type に変換)
     ↓
   Fragment Shader (重心座標で内挿、is_line 判定)
```

## フェーズ 1: Rust プロセッシング（VectorVertexBuilder）

### 1.1 フォント解析

TTF フォントの "E" グリフは rustybuzz により OutlineBuilder インターフェースを通じて解析されます：

```rust
// 典型的な "E" のアウトライン
move_to(100, 0)      // 左下スタート
line_to(100, 100)    // 左側縦線
line_to(300, 100)    // 右側上部
line_to(300, 80)     // 上部深さ調整
line_to(120, 80)     // 左側に戻る
line_to(120, 50)     // 中央線開始位置
line_to(280, 50)     // 中央線右端 ← **この line_to が直線三角形を生成**
line_to(280, 30)     // 中央線深さ調整
...
close()              // パス終了
```

### 1.2 move_to での頂点生成

```rust
pub fn move_to(&mut self, x: f32, y: f32) {
    let wait = self.next_wait();  // FlipFlop::Flip または Flop
    
    // 頂点1: 始点（通常版）
    self.vertex.push(InternalVertex { 
        x, y, 
        wait  // Flip or Flop
    });
    
    // 頂点2: 始点（直線版）
    self.vertex.push(InternalVertex { 
        x, y, 
        wait: wait.for_line()  // FlipForLine or FlopForLine
    });
    
    self.path_start_index = Some(self.current_index);
    self.current_index += 2;
}
```

状態遷移：
- `next_wait()` は FlipFlop::Flip ↔ Flop を順番に返す
- Flip なら次は Flop、Flop なら次は Flip

### 1.3 line_to での頂点と三角形生成

```rust
pub fn line_to(&mut self, x: f32, y: f32) {
    let wait = self.next_wait();  // 前の状態から遷移
    
    // **新しい直線の始点**
    self.vertex.push(InternalVertex { 
        x, y, 
        wait  // Flip or Flop
    });
    
    // **直線の終点（直線専用版）**
    self.vertex.push(InternalVertex { 
        x, y, 
        wait: wait.for_line()  // FlipForLine or FlopForLine
    });
    
    // **三角形インデックスの生成**
    self.index.push(1);                  // 原点L の固定インデックス
    self.index.push(self.current_index);     // 始点B (current_index)
    self.index.push(self.current_index + 2); // 終点L (current_index + 2)
    
    self.current_index += 2;
}
```

**重要**: line_to で生成される三角形は3回の line_to ごとに異なる頂点を使用します。

### 1.4 FlipFlop → vertex_type 変換表

| FlipFlop 状態 | vertex_type | 説明 | 親クラス |
|:---|:---:|:---|:---|
| Flip | 2 | 始点B（ベジエ開始点） | OutlineBuilder move_to/line_to の始点 |
| FlipForLine | 3 | 始点L（直線専用） | line_to での始点 |
| Flop | 4 | 終点B（ベジエ終点） | 通常の終点 |
| FlopForLine | 5 | 終点L（直線専用） | line_to での終点 |
| Control | 6 | 制御点 | quad_to の制御点 |
| (未設定) | 0 | 原点B（ベジエ原点） | インデックスの固定値 |
| (未設定) | 1 | 原点L（直線原点） | インデックスの固定値 |

## フェーズ 2: 直線三角形の構成

### 2.1 "E" の中央線の典型的な三角形構成

```
中央線（line_to で生成）の例：

move_to(120, 50) → 頂点インデックス: [2, 3]
line_to(280, 50) → 頂点インデックス: [4, 5]
                    **生成される三角形のインデックス: [1, 4, 5]**
```

### 2.2 三角形の3つの頂点構成

直線三角形（is_line）は常に以下の構成：

```
三角形 = { 原点L, 始点B, 終点L }

頂点A: 原点L (インデックス 1)
  - vertex_type = 1
  - Rust側の待機値アサイン: wait = [1, 0, 0]
  - 元のFlipFlop: (特定の値ではなく固定アサイン)

頂点B: 始点B (インデックス n)
  - vertex_type = 2 (Flip)
  - Rust側の待機値アサイン: wait = [0, 1, 0]
  - 元のFlipFlop: Flip

頂点C: 終点L (インデックス n+2)
  - vertex_type = 5 (FlopForLine)
  - Rust側の待機値アサイン: wait = [0, 0, 1]
  - 元のFlipFlop: FlopForLine
```

### 2.3 複数の直線三角形の連結

"E" の中央線が複雑な形状の場合、複数の line_to により複数の直線三角形が生成：

```
line_to(P1) → 三角形 T1 = [原点L, 始点1, 終点1]
line_to(P2) → 三角形 T2 = [原点L, 始点2, 終点2]
line_to(P3) → 三角形 T3 = [原点L, 始点3, 終点3]
...

各三角形は同じ "原点L" を共有するため、連続性が保証される
```

## フェーズ 3: シェーダー処理

### 3.1 Vertex Shader での wait と triangle_type の設定

各頂点の vertex_type に基づいて、待機値と三角形タイプが設定されます：

```wgsl
if model.vertex_type == 1u {
    // 原点L (直線三角形の原点)
    out.wait = vec3<f32>(1.0, 0.0, 0.0);        // 第1座標が 1.0
    out.triangle_type = vec3<f32>(0.0, 0.0, 1.0); // Z = 1.0 → is_line
} else if model.vertex_type == 2u {
    // 始点B (直線の始点)
    out.wait = vec3<f32>(0.0, 1.0, 0.0);        // 第2座標が 1.0
    out.triangle_type = vec3<f32>(1.0, 1.0, 0.0); // Bezier + Bezier Line
} else if model.vertex_type == 5u {
    // 終点L (直線の終点)
    out.wait = vec3<f32>(0.0, 0.0, 1.0);        // 第3座標が 1.0
    out.triangle_type = vec3<f32>(0.0, 0.0, 1.0); // Z = 1.0 → is_line
}
```

### 3.2 重心座標の内挿

GPU の自動内挿により、三角形内のピクセルで wait 値が計算されます：

```
直線三角形内のピクセル P での wait 値：

wait(P) = u × wait_A + v × wait_B + w × wait_C
        = u × [1, 0, 0] + v × [0, 1, 0] + w × [0, 0, 1]
        = [u, v, w]

ただし u + v + w = 1.0 (重心座標の性質)

例）三角形の中心 (u≈0.33, v≈0.33, w≈0.33):
  wait ≈ [0.33, 0.33, 0.33]
```

### 3.3 Fragment Shader での is_line 判定

```wgsl
fn fs_main(in: VertexOutput) -> FragmentOutput {
    // 三角形タイプの判定
    let is_bezier_pre = near_eq_one(in.triangle_type.x);       // X ≈ 1.0?
    let is_bezier_line_pre = near_eq_one(in.triangle_type.y);  // Y ≈ 1.0?
    let is_line_pre = near_eq_one(in.triangle_type.z);         // Z ≈ 1.0?
    
    // 排他的論理積で正確な型を判定
    let is_line = is_line_pre && !is_bezier_pre && !is_bezier_line_pre;
    
    // is_line = true の場合のピクセル判定
    if is_line {
        // **直線ピクセルの条件**
        if (in_naive_range(in.wait.y)) && (in_naive_range(in.wait.z)) {
            output.count.r = UNIT;
            // アンチエイリアス（AA）処理
            if !near_eq_one(alpha) {
                output.count.g = alpha / ALPHA_STEP;
                output.count.b = UNIT;
            }
        }
    }
}
```

### 3.4 in_naive_range() 関数

```wgsl
fn in_naive_range(value: f32) -> bool {
    return value >= 0.0 && value <= 1.0;  // [0, 1] 範囲？
}
```

**直線ピクセルの判定ロジック**：
- wait.y >= 0 && wait.y <= 1.0：始点B の重心座標が正の値
- wait.z >= 0 && wait.z <= 1.0：終点L の重心座標が正の値
- 両方満たす → 三角形内のピクセル（直線を通るピクセル）

重心座標の性質から u + v + w = 1.0 なので：
- wait.y と wait.z が両方 [0, 1] なら、自動的に wait.x も [0, 1]

## wait 値の詳細レジスタリング

### 4.1 各頂点タイプでの wait オフセット

三角形のどのエッジやコーナーにどのような待機値が来るかによって描画判定が変わります：

| 頂点タイプ | wait 値 | 重心座標での役割 | 直線判定への関与 |
|:---|:---|:---|:---|
| 原点L (1) | [1.0, 0.0, 0.0] | u=1 (頂点A) | 判定対象外（条件外） |
| 始点B (2) | [0.0, 1.0, 0.0] | v=1 (頂点B) | ✓ wait.y = 1.0 で有効 |
| 終点L (5) | [0.0, 0.0, 1.0] | w=1 (頂点C) | ✓ wait.z = 1.0 で有効 |

### 4.2 直線三角形内の wait 値の空間分布

```
平面図（直線三角形内）:

        原点L
        (1,0,0)
           /\
          /  \
         /    \  wait.y: 0→1
        /      \
       /________\
   (0,1,0)   (0,0,1)
  始点B      終点L

wait.x: 1 → 0 （上から下へ）
wait.y: 0 → 1 （左から右へ）
wait.z: 0 → 1 （左から右へ）

直線判定 in_naive_range(wait.y) && in_naive_range(wait.z):
  → 三角形全体がこの条件を満たす（重心座標の性質）
```

### 4.3 フローの実装詳細

```rust
// Rust側（build 時）での wait 割り当てパターン

// 直線三角形の場合、毎回必ずこのパターン：
//   move_to 後 1回目の line_to → インデックス [1, idx, idx+2]
//   wait_A (原点L)  = [1.0, 0.0, 0.0]
//   wait_B (始点B)  = [0.0, 1.0, 0.0]
//   wait_C (終点L)  = [0.0, 0.0, 1.0]

// 状態遷移（next_wait()）により flip-flop する：
//   wait Flip  → FlipForLine (3 → 三角形組) の直線版
//   wait Flop  → FlopForLine (5 → 三角形組) の直線版
```

## 直線判定の検証ロジック

### 5.1 三角形内のピクセル確認フロー

```
for each pixel P in triangle:
    P の重心座標 (u, v, w) を計算
    wait_P = [u, v, w]
    
    // is_line 判定
    if triangle_type.z ≈ 1.0:  // is_line_pre
        if triangle_type.x ≈ 0.0 && triangle_type.y ≈ 0.0:  // Bezier でない
            is_line = true
            
            // ピクセル有効化判定
            if wait_P.y in [0, 1] && wait_P.z in [0, 1]:
                // ← 直線三角形では常に true
                output.count.r = UNIT  // ピクセル描画
```

### 5.2 アンチエイリアス（AA）処理

直線ピクセルに対して、エッジでのアンチエイリアス：

```wgsl
// 直線の SDF 距離
let triangle_distance = in.wait.x;

// 隣接ピクセルの距離との差分
let triangle_distance_fwidth = fwidth(triangle_distance);

// AA 有効時：スムーズに減衰
var alpha = linerstep(
    -triangle_distance_fwidth / 2.0,
    triangle_distance_fwidth / 2.0,
    triangle_distance
);

// AA 無効時：0 または 1
if !enable_antialiasing {
    alpha = 1.0;
}
```

## 実装の重要ポイント

### ✓ wait 値の設定は Vertex Shader で行われる
- Rust では FlipFlop enum で状態を管理
- Vertex Shader で vertex_type → wait に変換

### ✓ 重心座標による自動内挿
- Fragment Shader では GPU が重心座標を計算
- wait 値は三角形内で線形補間される

### ✓ 直線判定は coordinate-based
- triangle_type.z ≈ 1.0 で直線と判定
- wait.y と wait.z の範囲チェックで確定

### ✓ 複数直線の連結
- 複数の line_to により複数の直線三角形が生成
- 全てが同じ "原点L" を共有し連続性を保証

## 参考ファイル

- [vector_vertex.rs](../../font_rasterizer/src/vector_vertex.rs): Rust 側頂点生成
- [overlap_shader.wgsl](../../font_rasterizer/src/shader/overlap_shader.wgsl): WGSL シェーダー
- [svg.rs](../../font_rasterizer/src/svg.rs): SVG / TTF 処理
