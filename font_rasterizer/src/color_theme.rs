#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) enum ColorMode {
    SolarizedLight,
    SolarizedDark,
}

#[allow(dead_code)]
impl ColorMode {
    pub(crate) fn text(&self) -> SolarizedColor {
        match self {
            ColorMode::SolarizedLight => SolarizedColor::Base00,
            ColorMode::SolarizedDark => SolarizedColor::Base0,
        }
    }

    pub(crate) fn text_comment(&self) -> SolarizedColor {
        match self {
            ColorMode::SolarizedLight => SolarizedColor::Base1,
            ColorMode::SolarizedDark => SolarizedColor::Base01,
        }
    }

    pub(crate) fn text_emphasized(&self) -> SolarizedColor {
        match self {
            ColorMode::SolarizedLight => SolarizedColor::Base01,
            ColorMode::SolarizedDark => SolarizedColor::Base1,
        }
    }

    pub(crate) fn background(&self) -> SolarizedColor {
        match self {
            ColorMode::SolarizedLight => SolarizedColor::Base3,
            ColorMode::SolarizedDark => SolarizedColor::Base03,
        }
    }

    pub(crate) fn background_highlights(&self) -> SolarizedColor {
        match self {
            ColorMode::SolarizedLight => SolarizedColor::Base2,
            ColorMode::SolarizedDark => SolarizedColor::Base02,
        }
    }

    pub(crate) fn yellow(&self) -> SolarizedColor {
        SolarizedColor::Yellow
    }
    pub(crate) fn orange(&self) -> SolarizedColor {
        SolarizedColor::Orange
    }
    pub(crate) fn red(&self) -> SolarizedColor {
        SolarizedColor::Red
    }
    pub(crate) fn magenta(&self) -> SolarizedColor {
        SolarizedColor::Magenta
    }
    pub(crate) fn violet(&self) -> SolarizedColor {
        SolarizedColor::Violet
    }

    pub(crate) fn blue(&self) -> SolarizedColor {
        SolarizedColor::Blue
    }
    pub(crate) fn cyan(&self) -> SolarizedColor {
        SolarizedColor::Cyan
    }
    pub(crate) fn green(&self) -> SolarizedColor {
        SolarizedColor::Green
    }
}

#[allow(dead_code)]
pub(crate) enum SolarizedColor {
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
    pub(crate) fn get_color(&self) -> [f32; 3] {
        match self {
            SolarizedColor::Base03 => [0.0030352699, 0.0241576303, 0.0368894450],
            SolarizedColor::Base02 => [0.0065120910, 0.0368894450, 0.0544802807],
            SolarizedColor::Base01 => [0.0975873619, 0.1559264660, 0.1778884083],
            SolarizedColor::Base00 => [0.1301364899, 0.1980693042, 0.2269658893],
            SolarizedColor::Base0 => [0.2269658893, 0.2961383164, 0.3049873710],
            SolarizedColor::Base1 => [0.2917706966, 0.3564002514, 0.3564002514],
            SolarizedColor::Base2 => [0.8549926877, 0.8069523573, 0.6653873324],
            SolarizedColor::Base3 => [0.9822505713, 0.9215820432, 0.7681512833],
            SolarizedColor::Yellow => [0.4620770514, 0.2501583695, 0.0000000000],
            SolarizedColor::Orange => [0.5972018838, 0.0703601092, 0.0080231922],
            SolarizedColor::Red => [0.7156936526, 0.0318960287, 0.0284260381],
            SolarizedColor::Magenta => [0.6514056921, 0.0368894450, 0.2232279778],
            SolarizedColor::Violet => [0.1499598026, 0.1651322246, 0.5520114899],
            SolarizedColor::Blue => [0.0193823613, 0.2581829131, 0.6444797516],
            SolarizedColor::Cyan => [0.0231533647, 0.3564002514, 0.3139887452],
            SolarizedColor::Green => [0.2345506549, 0.3185468316, 0.0000000000],
        }
    }
}

#[cfg(test)]
mod test {

    // good color scheme.
    // https://ethanschoonover.com/solarized/
    const SCHEMES: [(&str, u32, u32, u32); 16] = [
        ("base03", 10, 43, 54),
        ("base02", 19, 54, 66),
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
                scheme.0.to_uppercase(),
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
        println!("}};");

        SCHEMES.iter().for_each(|scheme| {
            println!(
                "SolarizedColor::{:10} => [{:.10}, {:.10}, {:.10}],",
                scheme.0.to_uppercase(),
                linear_to_srgb(scheme.1),
                linear_to_srgb(scheme.2),
                linear_to_srgb(scheme.3)
            );
        });
        println!("}};");
    }

    // こちらの記事を参考に linear の RGB 情報を sRGB に変換
    // http://www.psy.ritsumei.ac.jp/~akitaoka/RGBtoXYZ_etal01.html
    // https://en.wikipedia.org/wiki/SRGB
    fn linear_to_srgb(value: u32) -> f32 {
        let value: f32 = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
}
