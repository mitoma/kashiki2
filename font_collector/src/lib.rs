pub mod convert_text;

use std::path::PathBuf;

use convert_text::PreferredLanguage;
use log::{info, warn};

#[derive(Debug, Clone)]
pub struct FontData {
    pub font_name: String,
    pub binary: Vec<u8>,
    pub index: u32,
}

#[derive(Default)]
pub struct FontRepository {
    font_collector: FontCollector,
    primary_font: Option<FontData>,
    fallback_fonts: Vec<FontData>,
}

impl FontRepository {
    pub fn new(font_collector: FontCollector) -> Self {
        Self {
            font_collector,
            primary_font: None,
            fallback_fonts: Vec::new(),
        }
    }

    // 効率的ではないが所有権を頑張ると大変なので一旦ここはあきらめる
    pub fn get_fonts(&self) -> Vec<FontData> {
        let mut fonts = Vec::new();
        if let Some(font) = self.primary_font.as_ref() {
            fonts.push(font.clone());
        }
        for fallback_font in &self.fallback_fonts {
            fonts.push(fallback_font.clone());
        }
        fonts
    }

    pub fn set_primary_font(&mut self, font_name: &str) {
        if let Some(font_data) = self.font_collector.load_font(font_name) {
            self.primary_font = Some(font_data);
        }
    }

    pub fn add_fallback_font_from_system(&mut self, font_name: &str) {
        if let Some(font_data) = self.font_collector.load_font(font_name) {
            self.fallback_fonts.push(font_data);
        }
    }

    pub fn add_fallback_font_from_binary(&mut self, data: Vec<u8>, font_name: Option<String>) {
        if let Some(font_data) = self.font_collector.convert_font(data, font_name) {
            self.fallback_fonts.push(font_data);
        }
    }
}

pub struct FontCollector {
    font_paths: Vec<PathBuf>,
    preffered_language: Option<PreferredLanguage>,
}

impl Default for FontCollector {
    fn default() -> Self {
        Self {
            font_paths: Vec::new(),
            preffered_language: Some(PreferredLanguage::Japanese),
        }
    }
}

impl FontCollector {
    pub fn add_system_fonts(&mut self) {
        self.font_paths.push(system_font_dir());
    }

    pub fn add_font_path(&mut self, path: PathBuf) {
        self.font_paths.push(path);
    }

    fn list_font_files(&self) -> Vec<PathBuf> {
        let mut fonts = Vec::new();

        for font_path in &self.font_paths {
            if !font_path.exists() {
                warn!("font_path:{:?} not found", font_path);
                continue;
            }
            if font_path.is_file() {
                fonts.push(font_path.clone());
                continue;
            }
            for entry in std::fs::read_dir(font_path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() {
                    fonts.push(path);
                }
            }
        }
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
            info!("font_path:{:?}, names:{:?}", font_path, font_names);
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

    pub fn load_font(&self, font_name: &str) -> Option<FontData> {
        self.list_fonts()
            .iter()
            .map(|f| (f, f.font_index(font_name)))
            .filter(|(_, idx)| idx.is_some())
            .map(|(f, idx)| FontData {
                font_name: String::from(font_name),
                binary: f.data(),
                index: idx.unwrap(),
            })
            .next()
    }

    pub fn convert_font(&self, data: Vec<u8>, font_name: Option<String>) -> Option<FontData> {
        let ref_font = font_name.as_ref();
        let names = font_names_from_data(&data, self.preffered_language);
        names
            .into_iter()
            .enumerate()
            .find(|(_idx, name)| ref_font.map_or(true, |f_name| f_name == name))
            .map(|(idx, name)| FontData {
                font_name: name,
                binary: data,
                index: idx as u32,
            })
    }
}

fn system_font_dir() -> PathBuf {
    PathBuf::from("C:\\Windows\\Fonts")
}

fn font_names(font_path: &PathBuf, preferred_language: Option<PreferredLanguage>) -> Vec<String> {
    let data = std::fs::read(font_path).unwrap();
    font_names_from_data(data.as_slice(), preferred_language)
}

fn font_names_from_data(data: &[u8], preferred_language: Option<PreferredLanguage>) -> Vec<String> {
    let mut font_names = Vec::new();
    let names = convert_text::font_name(data, preferred_language);
    for name in names {
        font_names.push(name);
    }
    info!("font_name_length:{}", font_names.len());
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

    fn font_index(&self, font_name: &str) -> Option<u32> {
        self.names()
            .iter()
            .position(|name| name == font_name)
            .map(|idx| idx as u32)
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
        let mut collector = FontCollector::default();
        collector.add_system_fonts();
        collector.list_font_names().iter().for_each(|name| {
            println!("font_name:{:?}", name,);
        });
    }

    #[test]
    fn test_list_fonts2() {
        let mut collector = FontCollector::default();
        collector.add_font_path(PathBuf::from("../fonts"));
        assert_eq!(collector.list_font_names().len(), 2);
    }
}
