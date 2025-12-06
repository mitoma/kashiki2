#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorTheme {
    SolarizedLight,
    SolarizedDark,
    SolarizedBlackback,
    HighContrastLight,
    HighContrastDark,
    WarmLight,
    WarmDark,
    CoolLight,
    CoolDark,
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

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::Custom { r, g, b }
    }
}

impl From<&str> for Color {
    // #RRGGBB 形式の文字列を Color に変換する
    fn from(value: &str) -> Self {
        let value = value.trim();
        let value = value.strip_prefix('#').unwrap_or(value);

        if value.len() != 6 {
            // 不正な形式の場合は黒色を返す
            return Self::Custom { r: 0, g: 0, b: 0 };
        }

        let r = u8::from_str_radix(&value[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&value[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&value[4..6], 16).unwrap_or(0);

        Self::Custom { r, g, b }
    }
}

#[derive(Clone, Copy)]
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

#[allow(dead_code)]
impl ColorTheme {
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
                text: "#000000".into(),
                text_comment: "#606060".into(),
                text_emphasized: "#000000".into(),
                background: "#FFFFFF".into(),
                background_highlights: "#F0F0F0".into(),
                yellow: "#B48200".into(),
                orange: "#C86400".into(),
                red: "#B40000".into(),
                magenta: "#B40078".into(),
                violet: "#643CB4".into(),
                blue: "#0050C8".into(),
                cyan: "#008CA0".into(),
                green: "#008C00".into(),
            },
            ColorTheme::HighContrastDark => ColorPalette {
                text: "#FFFFFF".into(),
                text_comment: "#C0C0C0".into(),
                text_emphasized: "#FFFFFF".into(),
                background: "#000000".into(),
                background_highlights: "#202020".into(),
                yellow: "#FFDC00".into(),
                orange: "#FFA032".into(),
                red: "#FF6464".into(),
                magenta: "#FF78DC".into(),
                violet: "#B48CFF".into(),
                blue: "#64B4FF".into(),
                cyan: "#50DCF0".into(),
                green: "#64FF64".into(),
            },
            ColorTheme::WarmLight => ColorPalette {
                text: "#503C32".into(),
                text_comment: "#8C7864".into(),
                text_emphasized: "#3C281E".into(),
                background: "#FAF5EB".into(),
                background_highlights: "#F0E6D2".into(),
                yellow: "#DCB400".into(),
                orange: "#E67828".into(),
                red: "#C83232".into(),
                magenta: "#C83C8C".into(),
                violet: "#8C50B4".into(),
                blue: "#3C64B4".into(),
                cyan: "#288C8C".into(),
                green: "#508C3C".into(),
            },
            ColorTheme::WarmDark => ColorPalette {
                text: "#F0E6D2".into(),
                text_comment: "#B4A591".into(),
                text_emphasized: "#FFF5E6".into(),
                background: "#1E1914".into(),
                background_highlights: "#2D261E".into(),
                yellow: "#FFDC50".into(),
                orange: "#FFA050".into(),
                red: "#FF7878".into(),
                magenta: "#FF8CC8".into(),
                violet: "#C8A0FF".into(),
                blue: "#78B4FF".into(),
                cyan: "#50DCFF".into(),
                green: "#8CDC78".into(),
            },
            ColorTheme::CoolLight => ColorPalette {
                text: "#1E3246".into(),
                text_comment: "#64788C".into(),
                text_emphasized: "#142337".into(),
                background: "#F0F5FA".into(),
                background_highlights: "#E6F0F8".into(),
                yellow: "#A08C00".into(),
                orange: "#B46428".into(),
                red: "#B43C50".into(),
                magenta: "#A03C8C".into(),
                violet: "#6450C8".into(),
                blue: "#0078DC".into(),
                cyan: "#00B4C8".into(),
                green: "#00A078".into(),
            },
            ColorTheme::CoolDark => ColorPalette {
                text: "#DCEBF5".into(),
                text_comment: "#96AABE".into(),
                text_emphasized: "#F0FAFF".into(),
                background: "#0F141E".into(),
                background_highlights: "#192530".into(),
                yellow: "#F0DC64".into(),
                orange: "#FFB464".into(),
                red: "#FF8CA0".into(),
                magenta: "#FF8CC8".into(),
                violet: "#C8A0FF".into(),
                blue: "#78B4FF".into(),
                cyan: "#50DCFF".into(),
                green: "#8CDC78".into(),
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

    pub fn text(&self) -> Color {
        self.palette().text
    }

    pub fn text_comment(&self) -> Color {
        self.palette().text_comment
    }

    pub fn text_emphasized(&self) -> Color {
        self.palette().text_emphasized
    }

    pub fn background(&self) -> Color {
        self.palette().background
    }

    pub fn background_highlights(&self) -> Color {
        self.palette().background_highlights
    }

    pub fn yellow(&self) -> Color {
        self.palette().yellow
    }

    pub fn orange(&self) -> Color {
        self.palette().orange
    }

    pub fn red(&self) -> Color {
        self.palette().red
    }

    pub fn magenta(&self) -> Color {
        self.palette().magenta
    }

    pub fn violet(&self) -> Color {
        self.palette().violet
    }

    pub fn blue(&self) -> Color {
        self.palette().blue
    }

    pub fn cyan(&self) -> Color {
        self.palette().cyan
    }

    pub fn green(&self) -> Color {
        self.palette().green
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
                #[cfg(not(target_family = "wasm"))]
                {
                    // Native: sRGB変換
                    let r = (*r as f32 / 255.0).powf(2.2);
                    let g = (*g as f32 / 255.0).powf(2.2);
                    let b = (*b as f32 / 255.0).powf(2.2);
                    [r, g, b]
                }
                #[cfg(target_family = "wasm")]
                {
                    // WASM: 線形RGB
                    let r = *r as f32 / 255.0;
                    let g = *g as f32 / 255.0;
                    let b = *b as f32 / 255.0;
                    [r, g, b]
                }
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
