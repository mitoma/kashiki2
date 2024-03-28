#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColorTheme {
    SolarizedLight,
    SolarizedDark,
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

#[allow(dead_code)]
impl ColorTheme {
    pub fn text(&self) -> SolarizedColor {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base00,
            ColorTheme::SolarizedDark => SolarizedColor::Base0,
        }
    }

    pub fn text_comment(&self) -> SolarizedColor {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base1,
            ColorTheme::SolarizedDark => SolarizedColor::Base01,
        }
    }

    pub fn text_emphasized(&self) -> SolarizedColor {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base01,
            ColorTheme::SolarizedDark => SolarizedColor::Base1,
        }
    }

    pub fn background(&self) -> SolarizedColor {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base3,
            ColorTheme::SolarizedDark => SolarizedColor::Base03,
        }
    }

    pub fn background_highlights(&self) -> SolarizedColor {
        match self {
            ColorTheme::SolarizedLight => SolarizedColor::Base2,
            ColorTheme::SolarizedDark => SolarizedColor::Base02,
        }
    }

    pub fn yellow(&self) -> SolarizedColor {
        SolarizedColor::Yellow
    }

    pub fn orange(&self) -> SolarizedColor {
        SolarizedColor::Orange
    }

    pub fn red(&self) -> SolarizedColor {
        SolarizedColor::Red
    }

    pub fn magenta(&self) -> SolarizedColor {
        SolarizedColor::Magenta
    }

    pub fn violet(&self) -> SolarizedColor {
        SolarizedColor::Violet
    }

    pub fn blue(&self) -> SolarizedColor {
        SolarizedColor::Blue
    }

    pub fn cyan(&self) -> SolarizedColor {
        SolarizedColor::Cyan
    }

    pub fn green(&self) -> SolarizedColor {
        SolarizedColor::Green
    }
}

#[allow(dead_code)]
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
}

impl SolarizedColor {
    #[allow(clippy::excessive_precision)]
    pub fn get_color(&self) -> [f32; 3] {
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
        }
    }
}

// テーマの色を取得するための列挙型
#[derive(Clone, Copy)]
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
    }

    // learn-wgpu の注釈を元に変換する
    fn linear_to_srgb(value: u32) -> f32 {
        (value as f64 / 255.0).powf(2.2) as f32
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
