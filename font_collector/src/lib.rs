pub mod convert_text;

use std::path::PathBuf;

use convert_text::PreferredLanguage;

fn font_dir() -> PathBuf {
    let buf = PathBuf::from("C:\\Windows\\Fonts");
    buf
}

fn list_font_files() -> Vec<PathBuf> {
    let mut fonts = Vec::new();
    let font_dir = font_dir();
    for entry in std::fs::read_dir(font_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            fonts.push(path);
        }
    }
    fonts
}

#[derive(Debug, Clone)]
pub enum Font {
    File(PathBuf, Vec<String>),
    InMemory(u16, Vec<String>),
}

impl Font {
    fn names(&self) -> Vec<String> {
        match self {
            Font::File(_path, names) => names.clone(),
            Font::InMemory(_id, names) => names.clone(),
        }
    }

    fn contains(&self, font_name: &str) -> bool {
        self.names().iter().any(|name| name.contains(font_name))
    }

    fn data(&self) -> Vec<u8> {
        match self {
            Font::File(path, _names) => std::fs::read(path).unwrap(),
            Font::InMemory(_id, _names) => todo!(),
        }
    }
}

fn font_names(font_path: &PathBuf, preferred_language: Option<PreferredLanguage>) -> Vec<String> {
    let mut font_names = Vec::new();
    let data = std::fs::read(font_path).unwrap();
    let names = convert_text::font_name(data.as_slice(), preferred_language);
    for name in names {
        font_names.push(name);
    }
    font_names
}

pub fn list_fonts() -> Vec<Font> {
    let mut fonts = Vec::new();
    let font_file_paths = list_font_files();
    for font_path in font_file_paths {
        let font_names = font_names(&font_path, Some(PreferredLanguage::Japanese));
        if font_names.is_empty() {
            continue;
        }
        fonts.push(Font::File(font_path.clone(), font_names));
    }
    fonts
}

pub fn load_font(font_name: &str) -> Option<Vec<u8>> {
    list_fonts()
        .iter()
        .filter(|f| f.contains(font_name))
        .map(|f| f.data())
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_fontss() {
        let fonts = list_fonts();
        let mut font_names = fonts
            .into_iter()
            .flat_map(|font| match font {
                Font::File(_path, names) => names,
                Font::InMemory(_id, names) => names,
            })
            .collect::<Vec<String>>();
        font_names.sort();
        font_names.iter().for_each(|name| {
            println!("font_name:{:?}", name,);
        });
    }
}
