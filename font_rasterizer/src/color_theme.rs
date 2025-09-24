#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorTheme {
    SolarizedLight,
    SolarizedDark,
    SolarizedBlackback,
    Custom {
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
    },
}

impl From<SolarizedColor> for wgpu::Color {
    fn from(value: SolarizedColor) -> Self {
        let [r, g, b] = value.get_color();
        Self {
            r: r as f64,
            g: g as f64,
            b: b as f64,
            a: 1.0,
        }
    }
}

impl From<SolarizedColor> for Color {
    fn from(value: SolarizedColor) -> Self {
        Self::Solarized(value)
    }
}

#[allow(dead_code)]
impl ColorTheme {
    pub fn text(&self) -> Color {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base00.into(),
            ColorTheme::SolarizedDark => SolarizedColor::Base0.into(),
            ColorTheme::SolarizedBlackback => SolarizedColor::Base0.into(),
            ColorTheme::Custom { text, .. } => *text,
        }
    }

    pub fn text_comment(&self) -> Color {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base1.into(),
            ColorTheme::SolarizedDark => SolarizedColor::Base01.into(),
            ColorTheme::SolarizedBlackback => SolarizedColor::Base01.into(),
            ColorTheme::Custom { text_comment, .. } => *text_comment,
        }
    }

    pub fn text_emphasized(&self) -> Color {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base01.into(),
            ColorTheme::SolarizedDark => SolarizedColor::Base1.into(),
            ColorTheme::SolarizedBlackback => SolarizedColor::Base1.into(),
            ColorTheme::Custom {
                text_emphasized, ..
            } => *text_emphasized,
        }
    }

    pub fn background(&self) -> Color {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base3.into(),
            ColorTheme::SolarizedDark => SolarizedColor::Base03.into(),
            ColorTheme::SolarizedBlackback => SolarizedColor::Black.into(),
            ColorTheme::Custom { background, .. } => *background,
        }
    }

    pub fn background_highlights(&self) -> Color {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base2.into(),
            ColorTheme::SolarizedDark => SolarizedColor::Base02.into(),
            ColorTheme::SolarizedBlackback => SolarizedColor::Base02.into(),
            ColorTheme::Custom {
                background_highlights,
                ..
            } => *background_highlights,
        }
    }

    pub fn yellow(&self) -> Color {
        match self {
            ColorTheme::Custom { yellow, .. } => *yellow,
            _ => SolarizedColor::Yellow.into(),
        }
    }

    pub fn orange(&self) -> Color {
        match self {
            ColorTheme::Custom { orange, .. } => *orange,
            _ => SolarizedColor::Orange.into(),
        }
    }

    pub fn red(&self) -> Color {
        match self {
            ColorTheme::Custom { red, .. } => *red,
            _ => SolarizedColor::Red.into(),
        }
    }

    pub fn magenta(&self) -> Color {
        match self {
            ColorTheme::Custom { magenta, .. } => *magenta,
            _ => SolarizedColor::Magenta.into(),
        }
    }

    pub fn violet(&self) -> Color {
        match self {
            ColorTheme::Custom { violet, .. } => *violet,
            _ => SolarizedColor::Violet.into(),
        }
    }

    pub fn blue(&self) -> Color {
        match self {
            ColorTheme::Custom { blue, .. } => *blue,
            _ => SolarizedColor::Blue.into(),
        }
    }

    pub fn cyan(&self) -> Color {
        match self {
            ColorTheme::Custom { cyan, .. } => *cyan,
            _ => SolarizedColor::Cyan.into(),
        }
    }

    pub fn green(&self) -> Color {
        match self {
            ColorTheme::Custom { green, .. } => *green,
            _ => SolarizedColor::Green.into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]

pub enum Color {
    Solarized(SolarizedColor),
    Custom { r: u8, g: u8, b: u8 },
}

impl From<Color> for wgpu::Color {
    fn from(value: Color) -> Self {
        match value {
            Color::Solarized(color) => color.into(),
            Color::Custom { r, g, b } => Self {
                r: r as f64 / 255.0,
                g: g as f64 / 255.0,
                b: b as f64 / 255.0,
                a: 1.0,
            },
        }
    }
}

impl Color {
    pub fn get_color(&self) -> [f32; 3] {
        match self {
            Color::Solarized(color) => color.get_color(),
            Color::Custom { r, g, b } => {
                let r = *r as f32 / 255.0;
                let g = *g as f32 / 255.0;
                let b = *b as f32 / 255.0;
                [r, g, b]
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SolarizedColor {
    Base03,
    Base02,
    Base01,
    Base00,
    Base0,
    Base1,
    Base2,
    Base3,
    Yellow,
    Orange,
    Red,
    Magenta,
    Violet,
    Blue,
    Cyan,
    Green,
    // SolarizedColor としては定義されていないが、完全な黒背景のために定義
    Black,
}

impl SolarizedColor {
    #[allow(clippy::excessive_precision)]
    pub fn get_color(&self) -> [f32; 3] {
        #[cfg(not(target_family = "wasm"))]
        match self {
            SolarizedColor::Base03 => [0.0000000000, 0.0199178383, 0.0328759179],
            SolarizedColor::Base02 => [0.0003671363, 0.0328759179, 0.0511220507],
            SolarizedColor::Base01 => [0.0962661207, 0.1572807282, 0.1801442951],
            SolarizedColor::Base00 => [0.1303522736, 0.2010957301, 0.2309981287],
            SolarizedColor::Base0 => [0.2309981287, 0.3021254838, 0.3111805022],
            SolarizedColor::Base1 => [0.2976526320, 0.3636038899, 0.3636038899],
            SolarizedColor::Base2 => [0.8591735959, 0.8122414947, 0.6730490923],
            SolarizedColor::Base3 => [0.9828262329, 0.9239933491, 0.7742273211],
            SolarizedColor::Yellow => [0.4704402387, 0.2549158633, 0.0000000000],
            SolarizedColor::Orange => [0.6054843068, 0.0677245930, 0.0045597549],
            SolarizedColor::Red => [0.7226724625, 0.0277552791, 0.0242229421],
            SolarizedColor::Magenta => [0.6592239738, 0.0328759179, 0.2271365225],
            SolarizedColor::Violet => [0.1510580480, 0.1668722779, 0.5604991317],
            SolarizedColor::Blue => [0.0151752383, 0.2631747127, 0.6523700953],
            SolarizedColor::Cyan => [0.0189129841, 0.3636038899, 0.3203815520],
            SolarizedColor::Green => [0.2388279885, 0.3250369728, 0.0000000000],
            SolarizedColor::Black => [0.0, 0.0, 0.0],
        }
        #[cfg(target_family = "wasm")]
        match self {
            SolarizedColor::Base03 => [0.0000000000, 0.1686274558, 0.2117647082],
            SolarizedColor::Base02 => [0.0274509806, 0.2117647082, 0.2588235438],
            SolarizedColor::Base01 => [0.3450980484, 0.4313725531, 0.4588235319],
            SolarizedColor::Base00 => [0.3960784376, 0.4823529422, 0.5137255192],
            SolarizedColor::Base0 => [0.5137255192, 0.5803921819, 0.5882353187],
            SolarizedColor::Base1 => [0.5764706135, 0.6313725710, 0.6313725710],
            SolarizedColor::Base2 => [0.9333333373, 0.9098039269, 0.8352941275],
            SolarizedColor::Base3 => [0.9921568632, 0.9647058845, 0.8901960850],
            SolarizedColor::Yellow => [0.7098039389, 0.5372549295, 0.0000000000],
            SolarizedColor::Orange => [0.7960784435, 0.2941176593, 0.0862745121],
            SolarizedColor::Red => [0.8627451062, 0.1960784346, 0.1843137294],
            SolarizedColor::Magenta => [0.8274509907, 0.2117647082, 0.5098039508],
            SolarizedColor::Violet => [0.4235294163, 0.4431372583, 0.7686274648],
            SolarizedColor::Blue => [0.1490196139, 0.5450980663, 0.8235294223],
            SolarizedColor::Cyan => [0.1647058874, 0.6313725710, 0.5960784554],
            SolarizedColor::Green => [0.5215686560, 0.6000000238, 0.0000000000],
            SolarizedColor::Black => [0.0, 0.0, 0.0],
        }
    }
}

// テーマの色を取得するための列挙型
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub enum ThemedColor {
    Text,
    TextComment,
    TextEmphasized,
    Background,
    BackgroundHighlights,
    Yellow,
    Orange,
    Red,
    Magenta,
    Violet,
    Blue,
    Cyan,
    Green,
}

impl ThemedColor {
    pub fn get_color(&self, theme: &ColorTheme) -> [f32; 3] {
        match self {
            ThemedColor::Text => theme.text().get_color(),
            ThemedColor::TextComment => theme.text_comment().get_color(),
            ThemedColor::TextEmphasized => theme.text_emphasized().get_color(),
            ThemedColor::Background => theme.background().get_color(),
            ThemedColor::BackgroundHighlights => theme.background_highlights().get_color(),
            ThemedColor::Yellow => theme.yellow().get_color(),
            ThemedColor::Orange => theme.orange().get_color(),
            ThemedColor::Red => theme.red().get_color(),
            ThemedColor::Magenta => theme.magenta().get_color(),
            ThemedColor::Violet => theme.violet().get_color(),
            ThemedColor::Blue => theme.blue().get_color(),
            ThemedColor::Cyan => theme.cyan().get_color(),
            ThemedColor::Green => theme.green().get_color(),
        }
    }

    // 選択時の反転色を取得する
    // 反転色の選定基準はわりと適当
    pub fn get_selection_color(&self, theme: &ColorTheme) -> [f32; 3] {
        match self {
            ThemedColor::Text => theme.blue().get_color(),
            ThemedColor::TextComment => theme.text().get_color(),
            ThemedColor::TextEmphasized => theme.text_comment().get_color(),
            ThemedColor::Background => theme.background_highlights().get_color(),
            ThemedColor::BackgroundHighlights => theme.background().get_color(),
            ThemedColor::Yellow => theme.blue().get_color(),
            ThemedColor::Orange => theme.magenta().get_color(),
            ThemedColor::Red => theme.cyan().get_color(),
            ThemedColor::Magenta => theme.orange().get_color(),
            ThemedColor::Violet => theme.green().get_color(),
            ThemedColor::Blue => theme.yellow().get_color(),
            ThemedColor::Cyan => theme.red().get_color(),
            ThemedColor::Green => theme.violet().get_color(),
        }
    }
}

#[cfg(test)]
mod test {

    // good color scheme.
    // https://ethanschoonover.com/solarized/
    const SCHEMES: [(&str, u32, u32, u32); 16] = [
        ("base03", 0, 43, 54),
        ("base02", 7, 54, 66),
        ("base01", 88, 110, 117),
        ("base00", 101, 123, 131),
        ("base0", 131, 148, 150),
        ("base1", 147, 161, 161),
        ("base2", 238, 232, 213),
        ("base3", 253, 246, 227),
        ("yellow", 181, 137, 0),
        ("orange", 203, 75, 22),
        ("red", 220, 50, 47),
        ("magenta", 211, 54, 130),
        ("violet", 108, 113, 196),
        ("blue", 38, 139, 210),
        ("cyan", 42, 161, 152),
        ("green", 133, 153, 0),
    ];

    #[test]
    fn generate_color_table_for_wgsl() {
        SCHEMES.iter().for_each(|scheme| {
            println!(
                "let {:10} = vec4<f32>({:.10}, {:.10}, {:.10}, 1.0);",
                scheme.0,
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
    }

    #[test]
    fn generate_color_table_for_rust_enum() {
        println!("pub(crate) enum SolarizedColor {{");
        SCHEMES.iter().for_each(|scheme| {
            println!(
                "{:10}({:.10}, {:.10}, {:.10}),",
                scheme.0,
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
        println!("}};");

        SCHEMES.iter().for_each(|scheme| {
            println!(
                "SolarizedColor::{:10} => [{:.10}, {:.10}, {:.10}],",
                scheme.0,
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
        println!("}};");

        println!("for wasm32 (WebGL)");
        SCHEMES.iter().for_each(|scheme| {
            println!(
                "SolarizedColor::{:10} => [{:.10}, {:.10}, {:.10}],",
                scheme.0,
                linear(scheme.1),
                linear(scheme.2),
                linear(scheme.3)
            );
        });
        println!("}};");
    }

    // learn-wgpu の注釈を元に変換する
    fn linear_to_srgb(value: u32) -> f32 {
        (value as f64 / 255.0).powf(2.2) as f32
    }

    // wasm32 の時は linear のほうが適切っぽい
    fn linear(value: u32) -> f32 {
        (value as f64 / 255.0) as f32
    }

    // こちらの記事を参考に linear の RGB 情報を sRGB に変換
    // http://www.psy.ritsumei.ac.jp/~akitaoka/RGBtoXYZ_etal01.html
    // https://en.wikipedia.org/wiki/SRGB
    #[allow(dead_code)]
    fn linear_to_srgb_bak(value: u32) -> f32 {
        let value: f32 = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
}
