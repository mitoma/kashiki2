# カラーテーマ拡張計画

## 概要

既存の Solarized テーマに加えて、以下の3種類のカラーテーマを追加します:
- **ハイコントラスト**: アクセシビリティ重視、視認性を最大化
- **暖色系**: 赤/オレンジ/黄を強調した温かみのある配色
- **寒色系**: 青/シアン/緑を強調した涼しげな配色

各カテゴリで Light/Dark バリアントを用意し、合計6つの新規テーマを実装します。

## 実装手順

### Step 1: ColorTheme enum への新規バリアント追加

**ファイル**: `font_rasterizer/src/color_theme.rs`

`ColorTheme` enum に以下の6つのバリアントを追加:

```rust
pub enum ColorTheme {
    // 既存
    SolarizedLight,
    SolarizedDark,
    SolarizedBlackback,
    
    // 新規追加
    HighContrastLight,
    HighContrastDark,
    WarmLight,
    WarmDark,
    CoolLight,
    CoolDark,
    
    // 既存
    Custom { ... },
}
```

### Step 2: 各メソッドへの色定義追加

**ファイル**: `font_rasterizer/src/color_theme.rs`

以下の13個のメソッドすべてに、新規6テーマのパターンマッチングを追加:

- `text()`
- `text_comment()`
- `text_emphasized()`
- `background()`
- `background_highlights()`
- `yellow()`
- `orange()`
- `red()`
- `magenta()`
- `violet()`
- `blue()`
- `cyan()`
- `green()`

色の指定は `Color::Custom { r, g, b }` を使用し、`SolarizedColor` には依存しません。

### Step 3: テーマ切り替えアクションの更新

**ファイル**: `kashikishi/src/kashikishi_actions.rs` (142-145行目付近)

`SystemChangeTheme` アクションのマッチング処理に以下を追加:

```rust
"high-contrast-light" => ColorTheme::HighContrastLight,
"high-contrast-dark" => ColorTheme::HighContrastDark,
"warm-light" => ColorTheme::WarmLight,
"warm-dark" => ColorTheme::WarmDark,
"cool-light" => ColorTheme::CoolLight,
"cool-dark" => ColorTheme::CoolDark,
```

### Step 4: 色定義の設計

#### ハイコントラストテーマ

**目標**: WCAG AAA準拠 (コントラスト比 7:1 以上)

**HighContrastLight**:
- 背景: 純白 (255, 255, 255)
- テキスト: 純黒 (0, 0, 0)
- コメント: ダークグレー (96, 96, 96)
- 強調テキスト: ディープブラック (0, 0, 0)
- ハイライト背景: ライトグレー (240, 240, 240)
- アクセントカラー: 高彩度・中明度の色

**HighContrastDark**:
- 背景: 純黒 (0, 0, 0)
- テキスト: 純白 (255, 255, 255)
- コメント: ライトグレー (192, 192, 192)
- 強調テキスト: ブライトホワイト (255, 255, 255)
- ハイライト背景: ダークグレー (32, 32, 32)
- アクセントカラー: 高彩度・高明度の色

#### 暖色系テーマ

**目標**: 赤/オレンジ/黄を強調、温かみと活力を表現

**WarmLight**:
- 背景: アイボリー/クリーム系 (250, 245, 235)
- テキスト: ダークブラウン (80, 60, 50)
- コメント: ミディアムブラウン (140, 120, 100)
- ハイライト背景: ベージュ (240, 230, 210)
- Yellow: ゴールデンイエロー (220, 180, 0)
- Orange: バーントオレンジ (230, 120, 40)
- Red: ディープレッド (200, 50, 50)
- 他のアクセント: 暖色寄りに調整

**WarmDark**:
- 背景: ダークブラウン (30, 25, 20)
- テキスト: ライトベージュ (240, 230, 210)
- コメント: ミディアムベージュ (180, 165, 145)
- ハイライト背景: ミディアムブラウン (45, 38, 30)
- アクセント: より鮮やかな暖色系

#### 寒色系テーマ

**目標**: 青/シアン/緑を強調、落ち着きと集中を表現

**CoolLight**:
- 背景: ライトブルーグレー (240, 245, 250)
- テキスト: ディープネイビー (30, 50, 70)
- コメント: スレートグレー (100, 120, 140)
- ハイライト背景: アイスブルー (230, 240, 248)
- Blue: ブライトブルー (0, 120, 220)
- Cyan: ターコイズ (0, 180, 200)
- Green: ティールグリーン (0, 160, 120)
- 他のアクセント: 寒色寄りに調整

**CoolDark**:
- 背景: ディープネイビー (15, 20, 30)
- テキスト: アイスブルー (220, 235, 245)
- コメント: スカイグレー (150, 170, 190)
- ハイライト背景: ミッドナイトブルー (25, 35, 48)
- アクセント: より鮮やかな寒色系

## 影響を受けるファイル

### 必須の変更
1. `font_rasterizer/src/color_theme.rs`
   - `ColorTheme` enum に6バリアント追加
   - 13メソッドに各テーマの色定義追加

2. `kashikishi/src/kashikishi_actions.rs`
   - `SystemChangeTheme` アクションに6テーマ名追加

### オプション(デフォルトテーマ変更時)
3. `kashikishi/src/main.rs`
   - `COLOR_THEME` 定数の変更

4. `font_rasterizer/src/context.rs`
   - `StateContext` の `Default` 実装

## 実装の優先順位

1. **Phase 1** (最小限): ColorTheme enum 追加 + 基本色定義
2. **Phase 2** (推奨): テーマ切り替えアクション対応

## 備考

- 現在 `ColorTheme` は85箇所以上で参照されていますが、ほとんどはメソッド呼び出しのため、新規バリアント追加の影響は限定的です
- RGB値は `Color::Custom` を使用し、0-255の整数値で指定します
- 色空間変換は既存の実装を踏襲します(非WASM環境では sRGB のガンマ補正あり)
