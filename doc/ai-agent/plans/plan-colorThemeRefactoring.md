# カラーテーマのリファクタリング計画

## 現状の問題点

### 1. コードの分散と重複
- 各メソッド（13個）で9つのバリアントのマッチングを繰り返している
- 合計: 13メソッド × 9バリアント = 117箇所の分岐
- テーマごとの色定義が分散しており、一つのテーマの全体像が把握しにくい

### 2. 保守性の課題
- 新しいテーマ追加時に13箇所すべてのメソッドを修正する必要がある
- 一つのテーマの色を変更する際、複数のメソッドを横断的に修正する必要がある
- テーマの一貫性を保つのが困難

### 3. 可読性の問題
- `HighContrastLight` の全色定義を確認するには13個のメソッドを見る必要がある
- テーマ間での色の比較が困難

## リファクタリング案

### 案1: 内部的に ColorPalette 構造体を使用 【推奨】

#### 概要
テーマごとの色定義を一箇所にまとめ、各バリアントが `ColorPalette` 構造体を返すようにする。

#### 実装イメージ

```rust
// 内部用の構造体（pub にはしない）
struct ColorPalette {
    text: Color,
    text_comment: Color,
    text_emphasized: Color,
    background: Color,
    background_highlights: Color,
    yellow: Color,
    orange: Color,
    red: Color,
    magenta: Color,
    violet: Color,
    blue: Color,
    cyan: Color,
    green: Color,
}

impl ColorTheme {
    // 各テーマの色定義を一箇所に集約
    fn palette(&self) -> ColorPalette {
        match self {
            ColorTheme::SolarizedLight => ColorPalette {
                text: SolarizedColor::Base00.into(),
                text_comment: SolarizedColor::Base1.into(),
                text_emphasized: SolarizedColor::Base01.into(),
                background: SolarizedColor::Base3.into(),
                background_highlights: SolarizedColor::Base2.into(),
                yellow: SolarizedColor::Yellow.into(),
                orange: SolarizedColor::Orange.into(),
                red: SolarizedColor::Red.into(),
                magenta: SolarizedColor::Magenta.into(),
                violet: SolarizedColor::Violet.into(),
                blue: SolarizedColor::Blue.into(),
                cyan: SolarizedColor::Cyan.into(),
                green: SolarizedColor::Green.into(),
            },
            ColorTheme::SolarizedDark => ColorPalette {
                text: SolarizedColor::Base0.into(),
                text_comment: SolarizedColor::Base01.into(),
                text_emphasized: SolarizedColor::Base1.into(),
                background: SolarizedColor::Base03.into(),
                background_highlights: SolarizedColor::Base02.into(),
                yellow: SolarizedColor::Yellow.into(),
                orange: SolarizedColor::Orange.into(),
                red: SolarizedColor::Red.into(),
                magenta: SolarizedColor::Magenta.into(),
                violet: SolarizedColor::Violet.into(),
                blue: SolarizedColor::Blue.into(),
                cyan: SolarizedColor::Cyan.into(),
                green: SolarizedColor::Green.into(),
            },
            ColorTheme::SolarizedBlackback => ColorPalette {
                text: SolarizedColor::Base0.into(),
                text_comment: SolarizedColor::Base01.into(),
                text_emphasized: SolarizedColor::Base1.into(),
                background: SolarizedColor::Black.into(),
                background_highlights: SolarizedColor::Base02.into(),
                yellow: SolarizedColor::Yellow.into(),
                orange: SolarizedColor::Orange.into(),
                red: SolarizedColor::Red.into(),
                magenta: SolarizedColor::Magenta.into(),
                violet: SolarizedColor::Violet.into(),
                blue: SolarizedColor::Blue.into(),
                cyan: SolarizedColor::Cyan.into(),
                green: SolarizedColor::Green.into(),
            },
            ColorTheme::HighContrastLight => ColorPalette {
                text: Color::Custom { r: 0, g: 0, b: 0 },
                text_comment: Color::Custom { r: 96, g: 96, b: 96 },
                text_emphasized: Color::Custom { r: 0, g: 0, b: 0 },
                background: Color::Custom { r: 255, g: 255, b: 255 },
                background_highlights: Color::Custom { r: 240, g: 240, b: 240 },
                yellow: Color::Custom { r: 180, g: 130, b: 0 },
                orange: Color::Custom { r: 200, g: 100, b: 0 },
                red: Color::Custom { r: 180, g: 0, b: 0 },
                magenta: Color::Custom { r: 180, g: 0, b: 120 },
                violet: Color::Custom { r: 100, g: 60, b: 180 },
                blue: Color::Custom { r: 0, g: 80, b: 200 },
                cyan: Color::Custom { r: 0, g: 140, b: 160 },
                green: Color::Custom { r: 0, g: 140, b: 0 },
            },
            ColorTheme::HighContrastDark => ColorPalette {
                text: Color::Custom { r: 255, g: 255, b: 255 },
                text_comment: Color::Custom { r: 192, g: 192, b: 192 },
                text_emphasized: Color::Custom { r: 255, g: 255, b: 255 },
                background: Color::Custom { r: 0, g: 0, b: 0 },
                background_highlights: Color::Custom { r: 32, g: 32, b: 32 },
                yellow: Color::Custom { r: 255, g: 220, b: 0 },
                orange: Color::Custom { r: 255, g: 160, b: 50 },
                red: Color::Custom { r: 255, g: 100, b: 100 },
                magenta: Color::Custom { r: 255, g: 120, b: 220 },
                violet: Color::Custom { r: 180, g: 140, b: 255 },
                blue: Color::Custom { r: 100, g: 180, b: 255 },
                cyan: Color::Custom { r: 80, g: 220, b: 240 },
                green: Color::Custom { r: 100, g: 255, b: 100 },
            },
            ColorTheme::WarmLight => ColorPalette {
                text: Color::Custom { r: 80, g: 60, b: 50 },
                text_comment: Color::Custom { r: 140, g: 120, b: 100 },
                text_emphasized: Color::Custom { r: 60, g: 40, b: 30 },
                background: Color::Custom { r: 250, g: 245, b: 235 },
                background_highlights: Color::Custom { r: 240, g: 230, b: 210 },
                yellow: Color::Custom { r: 220, g: 180, b: 0 },
                orange: Color::Custom { r: 230, g: 120, b: 40 },
                red: Color::Custom { r: 200, g: 50, b: 50 },
                magenta: Color::Custom { r: 200, g: 60, b: 140 },
                violet: Color::Custom { r: 140, g: 80, b: 180 },
                blue: Color::Custom { r: 60, g: 100, b: 180 },
                cyan: Color::Custom { r: 40, g: 140, b: 140 },
                green: Color::Custom { r: 80, g: 140, b: 60 },
            },
            ColorTheme::WarmDark => ColorPalette {
                text: Color::Custom { r: 240, g: 230, b: 210 },
                text_comment: Color::Custom { r: 180, g: 165, b: 145 },
                text_emphasized: Color::Custom { r: 255, g: 245, b: 230 },
                background: Color::Custom { r: 30, g: 25, b: 20 },
                background_highlights: Color::Custom { r: 45, g: 38, b: 30 },
                yellow: Color::Custom { r: 255, g: 220, b: 80 },
                orange: Color::Custom { r: 255, g: 160, b: 80 },
                red: Color::Custom { r: 255, g: 120, b: 120 },
                magenta: Color::Custom { r: 255, g: 140, b: 200 },
                violet: Color::Custom { r: 200, g: 160, b: 255 },
                blue: Color::Custom { r: 120, g: 180, b: 255 },
                cyan: Color::Custom { r: 100, g: 220, b: 220 },
                green: Color::Custom { r: 140, g: 220, b: 120 },
            },
            ColorTheme::CoolLight => ColorPalette {
                text: Color::Custom { r: 30, g: 50, b: 70 },
                text_comment: Color::Custom { r: 100, g: 120, b: 140 },
                text_emphasized: Color::Custom { r: 20, g: 35, b: 55 },
                background: Color::Custom { r: 240, g: 245, b: 250 },
                background_highlights: Color::Custom { r: 230, g: 240, b: 248 },
                yellow: Color::Custom { r: 160, g: 140, b: 0 },
                orange: Color::Custom { r: 180, g: 100, b: 40 },
                red: Color::Custom { r: 180, g: 60, b: 80 },
                magenta: Color::Custom { r: 160, g: 60, b: 140 },
                violet: Color::Custom { r: 100, g: 80, b: 200 },
                blue: Color::Custom { r: 0, g: 120, b: 220 },
                cyan: Color::Custom { r: 0, g: 180, b: 200 },
                green: Color::Custom { r: 0, g: 160, b: 120 },
            },
            ColorTheme::CoolDark => ColorPalette {
                text: Color::Custom { r: 220, g: 235, b: 245 },
                text_comment: Color::Custom { r: 150, g: 170, b: 190 },
                text_emphasized: Color::Custom { r: 240, g: 250, b: 255 },
                background: Color::Custom { r: 15, g: 20, b: 30 },
                background_highlights: Color::Custom { r: 25, g: 35, b: 48 },
                yellow: Color::Custom { r: 240, g: 220, b: 100 },
                orange: Color::Custom { r: 255, g: 180, b: 100 },
                red: Color::Custom { r: 255, g: 140, b: 160 },
                magenta: Color::Custom { r: 240, g: 140, b: 220 },
                violet: Color::Custom { r: 160, g: 160, b: 255 },
                blue: Color::Custom { r: 100, g: 200, b: 255 },
                cyan: Color::Custom { r: 80, g: 240, b: 255 },
                green: Color::Custom { r: 100, g: 240, b: 200 },
            },
            ColorTheme::Custom {
                text,
                text_comment,
                text_emphasized,
                background,
                background_highlights,
                yellow,
                orange,
                red,
                magenta,
                violet,
                blue,
                cyan,
                green,
            } => ColorPalette {
                text: *text,
                text_comment: *text_comment,
                text_emphasized: *text_emphasized,
                background: *background,
                background_highlights: *background_highlights,
                yellow: *yellow,
                orange: *orange,
                red: *red,
                magenta: *magenta,
                violet: *violet,
                blue: *blue,
                cyan: *cyan,
                green: *green,
            },
        }
    }
    
    // 既存の public API は変更なし
    pub fn text(&self) -> Color {
        self.palette().text
    }
    
    pub fn text_comment(&self) -> Color {
        self.palette().text_comment
    }
    
    // ... 他の11メソッドも同様に1行で実装
}
```

#### メリット
1. **可読性の向上**: 各テーマの全色定義が一箇所にまとまる
2. **保守性の向上**: 新テーマ追加時は `palette()` メソッドに1ケースのみ追加すればよい
3. **互換性**: 既存のpublic APIは完全に保持、外部からの影響なし
4. **シンプルさ**: 複雑なマクロや配列インデックスを使わない、Rustの標準的なパターン
5. **型安全性**: 構造体のフィールド名で各色にアクセスできる

#### デメリット
1. **パフォーマンス**: 各メソッド呼び出しで `palette()` が実行される
   - ただし、ColorPalette は Copy trait を実装できるため影響は軽微
   - 色の取得は高頻度だがボトルネックにはならない程度
2. **メモリコピー**: 13個のColorをスタックにコピー
   - Color は Copy なので問題なし

#### パフォーマンス最適化案（必要な場合）
```rust
// キャッシュを追加する場合
impl ColorTheme {
    fn palette(&self) -> &'static ColorPalette {
        // 静的な配列からテーマに応じたパレットを返す
        // またはlazyな初期化を使う
    }
}
```

### 案2: const 関数で色定義を集約

#### 概要
各テーマの色定義を const 関数として定義し、配列でアクセスする。

#### 実装イメージ

```rust
impl ColorTheme {
    const fn high_contrast_light_colors() -> [Color; 13] {
        [
            Color::Custom { r: 0, g: 0, b: 0 },           // 0: text
            Color::Custom { r: 96, g: 96, b: 96 },        // 1: text_comment
            Color::Custom { r: 0, g: 0, b: 0 },           // 2: text_emphasized
            Color::Custom { r: 255, g: 255, b: 255 },     // 3: background
            Color::Custom { r: 240, g: 240, b: 240 },     // 4: background_highlights
            Color::Custom { r: 180, g: 130, b: 0 },       // 5: yellow
            Color::Custom { r: 200, g: 100, b: 0 },       // 6: orange
            Color::Custom { r: 180, g: 0, b: 0 },         // 7: red
            Color::Custom { r: 180, g: 0, b: 120 },       // 8: magenta
            Color::Custom { r: 100, g: 60, b: 180 },      // 9: violet
            Color::Custom { r: 0, g: 80, b: 200 },        // 10: blue
            Color::Custom { r: 0, g: 140, b: 160 },       // 11: cyan
            Color::Custom { r: 0, g: 140, b: 0 },         // 12: green
        ]
    }
    
    pub fn text(&self) -> Color {
        match self {
            ColorTheme::HighContrastLight => Self::high_contrast_light_colors()[0],
            // ... 他のテーマ
        }
    }
}
```

#### メリット
1. **コンパイル時定数**: 色定義がコンパイル時に評価される
2. **テーマ定義が関数単位で分離**: 各テーマが独立した const 関数

#### デメリット
1. **可読性の低下**: インデックスアクセスのため、どの色かわかりにくい
2. **型の不統一**: Solarized系は `SolarizedColor` 型なので統一しにくい
3. **保守性**: インデックスの管理が必要
4. **エラーの可能性**: インデックスを間違えるとバグの原因に

### 案3: マクロで重複を削減

#### 概要
マクロを使って各テーマの定義をコンパクトに記述する。

#### 実装イメージ

```rust
macro_rules! define_color_theme_palette {
    (
        $theme:ident => {
            text: $text:expr,
            text_comment: $text_comment:expr,
            text_emphasized: $text_emphasized:expr,
            background: $background:expr,
            background_highlights: $background_highlights:expr,
            yellow: $yellow:expr,
            orange: $orange:expr,
            red: $red:expr,
            magenta: $magenta:expr,
            violet: $violet:expr,
            blue: $blue:expr,
            cyan: $cyan:expr,
            green: $green:expr,
        }
    ) => {
        ColorTheme::$theme => ColorPalette {
            text: $text,
            text_comment: $text_comment,
            text_emphasized: $text_emphasized,
            background: $background,
            background_highlights: $background_highlights,
            yellow: $yellow,
            orange: $orange,
            red: $red,
            magenta: $magenta,
            violet: $violet,
            blue: $blue,
            cyan: $cyan,
            green: $green,
        }
    };
}

impl ColorTheme {
    fn palette(&self) -> ColorPalette {
        match self {
            define_color_theme_palette!(HighContrastLight => {
                text: Color::Custom { r: 0, g: 0, b: 0 },
                text_comment: Color::Custom { r: 96, g: 96, b: 96 },
                // ...
            }),
            // ... 他のテーマ
        }
    }
}
```

#### メリット
1. **コンパクトな定義**: 最も簡潔にテーマを定義できる
2. **重複の削減**: 構造体フィールド名の繰り返しを避けられる

#### デメリット
1. **マクロの複雑性**: マクロの理解とメンテナンスが必要
2. **デバッグ困難**: エラー時のメッセージがわかりにくい
3. **IDE支援の低下**: 補完やジャンプなどが効きにくい
4. **学習コスト**: チーム開発時に理解が必要

## 推奨案: 案1の採用

### 理由

1. **可読性**: 最も読みやすく、各テーマの全体像が一目でわかる
2. **保守性**: 新テーマ追加時は `palette()` に1ケースのみ追加すればよい
3. **互換性**: 既存のAPIは完全に保持、外部への影響なし
4. **シンプルさ**: Rustの標準的なパターン、特殊な知識不要
5. **型安全性**: 構造体のフィールド名で各色にアクセス、間違いにくい
6. **パフォーマンス**: Copy trait により軽微な影響のみ

### 実装手順

1. `ColorPalette` 構造体を定義（`#[derive(Clone, Copy)]` を付与）
2. `ColorTheme::palette()` メソッドを実装（全9テーマの色定義を集約）
3. 既存の13個のpublicメソッドを `self.palette().field_name` に書き換え
4. テストを実行して動作確認
5. 不要になった旧コードを削除

### コード行数の変化

- **現状**: 約400行（13メソッド × 各30行程度）
- **リファクタリング後**: 約250行
  - ColorPalette 定義: 15行
  - palette() メソッド: 200行（9テーマ × 各22行）
  - 13個のアクセサメソッド: 35行（各3行）

### 削減効果
- **約150行のコード削減**
- **117箇所の分岐 → 9箇所の分岐**に集約

## 追加の最適化案（オプション）

### キャッシング戦略（必要な場合）

頻繁に呼ばれる場合のキャッシング案：

```rust
use once_cell::sync::Lazy;
use std::collections::HashMap;

static THEME_CACHE: Lazy<HashMap<ColorTheme, ColorPalette>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(ColorTheme::SolarizedLight, ColorTheme::SolarizedLight.palette());
    // ... 他のテーマ
    map
});

impl ColorTheme {
    fn palette(&self) -> ColorPalette {
        if let Some(palette) = THEME_CACHE.get(self) {
            return *palette;
        }
        // Custom の場合のみ動的生成
        match self {
            ColorTheme::Custom { .. } => { /* ... */ }
            _ => unreachable!(),
        }
    }
}
```

**注意**: 現状のパフォーマンス特性では不要と判断。必要になった場合のみ導入を検討。

## まとめ

案1の `ColorPalette` 構造体を用いたリファクタリングが最適。シンプルで保守性が高く、既存のAPIとの互換性を保ちながら、コードの可読性と保守性を大幅に向上させることができる。
