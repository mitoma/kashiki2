pub mod convert_text;

use std::path::PathBuf;

use convert_text::PreferredLanguage;

pub struct FontCollector {
    font_paths: Vec<PathBuf>,
    preffered_language: Option<PreferredLanguage>,
}

impl FontCollector {
    pub fn new() -> Self {
        Self {
            font_paths: Vec::new(),
            preffered_language: Some(PreferredLanguage::Japanese),
        }
    }

    pub fn add_system_fonts(&mut self) {
        self.font_paths.push(system_font_dir());
    }

    pub fn add_font_path(&mut self, path: PathBuf) {
        self.font_paths.push(path);
    }

    fn list_font_files(&self) -> Vec<PathBuf> {
        let mut fonts = Vec::new();

        self.font_paths.iter().for_each(|font_path| {
            if font_path.is_file() {
                fonts.push(font_path.clone());
                return;
            }
            for entry in std::fs::read_dir(font_path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() {
                    fonts.push(path);
                }
            }
        });
        fonts
    }

    fn list_fonts(&self) -> Vec<Font> {
        let font_file_paths = self.list_font_files();
        let mut fonts = Vec::new();
        for font_path in font_file_paths {
            let font_names = font_names(&font_path, self.preffered_language);
            if font_names.is_empty() {
                continue;
            }
            fonts.push(Font::File(font_path.clone(), font_names));
        }
        fonts
    }

    pub fn list_font_names(&self) -> Vec<String> {
        let mut font_names = self
            .list_fonts()
            .into_iter()
            .flat_map(|font| match font {
                Font::File(_path, names) => names,
                Font::InMemory(_id, names) => names,
            })
            .collect::<Vec<String>>();
        font_names.sort();
        font_names
    }

    pub fn load_font(&self, font_name: &str) -> Option<Vec<u8>> {
        self.list_fonts()
            .iter()
            .filter(|f| f.contains(font_name))
            .map(|f| f.data())
            .next()
    }
}

fn system_font_dir() -> PathBuf {
    let buf = PathBuf::from("C:\\Windows\\Fonts");
    buf
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_fonts() {
        let mut collector = FontCollector::new();
        collector.add_system_fonts();
        collector.list_font_names().iter().for_each(|name| {
            println!("font_name:{:?}", name,);
        });
    }

    #[test]
    fn test_list_fonts2() {
        let mut collector = FontCollector::new();
        collector.add_font_path(PathBuf::from("../font_rasterizer/examples/font"));
        assert_eq!(collector.list_font_names().len(), 2);
    }
}
