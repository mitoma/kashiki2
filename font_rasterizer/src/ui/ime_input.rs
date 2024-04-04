use log::info;
use stroke_parser::Action;

use crate::{
    color_theme::ColorTheme,
    font_buffer::GlyphVertexBuffer,
    instances::GlyphInstances,
    motion::{CameraDetail, MotionFlags},
};

use super::single_line::SingleLine;

pub struct ImeInput {
    single_line: SingleLine,
}

impl Default for ImeInput {
    fn default() -> Self {
        Self::new()
    }
}

impl ImeInput {
    pub fn new() -> Self {
        let mut single_line = SingleLine::new("".to_string());
        single_line.update_motion(
            MotionFlags::builder()
                .camera_detail(CameraDetail::IGNORE_CAMERA)
                .build(),
        );
        single_line.update_scale([0.1, 0.1]);
        single_line.update_width(Some(1.0));
        Self { single_line }
    }

    pub fn apply_ime_event(&mut self, action: &Action) -> bool {
        match action {
            Action::ImePreedit(value, position) => {
                match position {
                    Some((start, end)) if start != end => {
                        info!("start:{start}, end:{end}");
                        let (first, center, last) =
                            split_preedit_string(value.clone(), *start, *end);
                        let preedit_str = format!("{}[{}]{}", first, center, last);
                        self.single_line.update_value(preedit_str);
                    }
                    _ => {
                        self.single_line.update_value(value.clone());
                    }
                };
                false
            }
            Action::ImeInput(_) => {
                self.single_line.update_value("".to_string());
                true
            }
            _ => false,
        }
    }

    pub fn update(
        &mut self,
        color_theme: &ColorTheme,
        glyph_vertex_buffer: &mut GlyphVertexBuffer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<&GlyphInstances> {
        self.single_line
            .generate_instances(color_theme, glyph_vertex_buffer, device, queue)
    }

    pub fn get_instances(&self) -> Vec<&GlyphInstances> {
        self.single_line.get_instances()
    }
}

enum Pos {
    First(char),
    Center(char),
    Last(char),
}

pub fn split_preedit_string(
    value: String,
    start_bytes: usize,
    end_bytes: usize,
) -> (String, String, String) {
    let splitted = value
        .chars()
        .scan(0_usize, |prev, c| {
            *prev += c.len_utf8();
            let prev = *prev;
            if prev <= start_bytes {
                Some(Pos::First(c))
            } else if prev <= end_bytes {
                Some(Pos::Center(c))
            } else {
                Some(Pos::Last(c))
            }
        })
        .collect::<Vec<_>>();
    let first: String = splitted
        .iter()
        .flat_map(|p| if let Pos::First(c) = p { Some(c) } else { None })
        .collect();
    let center: String = splitted
        .iter()
        .flat_map(|p| {
            if let Pos::Center(c) = p {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    let last: String = splitted
        .iter()
        .flat_map(|p| if let Pos::Last(c) = p { Some(c) } else { None })
        .collect();
    (first, center, last)
}

#[cfg(test)]
mod test {
    use super::split_preedit_string;

    #[test]
    fn test_split1() {
        test_split("こんにちは", 6, 12, ("こん", "にち", "は"));
        test_split("こんにちは", 0, 12, ("", "こんにち", "は"));
        test_split("こんにちは", 0, 15, ("", "こんにちは", ""));
        test_split("ABCDE", 2, 3, ("AB", "C", "DE"));
        test_split("AあBいCう", 4, 8, ("Aあ", "Bい", "Cう"));
    }

    fn test_split(target: &str, start: usize, end: usize, expects: (&str, &str, &str)) {
        let (first, center, last) = split_preedit_string(target.to_string(), start, end);
        assert_eq!(&first, expects.0);
        assert_eq!(&center, expects.1);
        assert_eq!(&last, expects.2);
    }
}
