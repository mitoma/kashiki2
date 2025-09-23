use std::{collections::HashSet, ops::Range};

use serde::Deserialize;

use crate::CallbackArguments;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HighlightSettings {
    pub definitions: Vec<HighlightCategoryDefinition>,
}

impl HighlightSettings {
    pub fn default() -> Self {
        Self::load_settings(&[
            include_str!("../asset/markdown.json"),
            include_str!("../asset/json.json"),
        ])
    }

    pub fn load_settings(setting_strings: &[&str]) -> Self {
        let mut result = HighlightSettings {
            definitions: vec![],
        };
        for setting_string in setting_strings {
            let defs: Vec<HighlightCategoryDefinition> =
                serde_json::from_str(setting_string).unwrap();
            result.definitions.extend(defs);
        }
        result
    }

    pub fn args_to_definition(&self, arg: &CallbackArguments) -> Option<(String, Range<usize>)> {
        for def in &self.definitions {
            for key_def in def.key_definitions.iter() {
                if def.language == arg.language {
                    if arg.kind_stack.ends_with(&key_def.key) {
                        return Some((def.name.clone(), arg.kind_stack.range(key_def.depth)));
                    }
                }
            }
        }
        None
    }

    pub fn categories(&self) -> Vec<String> {
        self.definitions
            .iter()
            .map(|d| &d.name)
            .collect::<HashSet<_>>()
            .into_iter()
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct HighlightCategoryDefinition {
    pub name: String,
    pub language: String,
    pub key_definitions: Vec<KeyDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct KeyDefinition {
    pub key: String,
    pub depth: usize,
}

#[cfg(test)]
mod tests {
    use crate::settings::HighlightSettings;

    #[test]
    fn test_load_definitions() {
        let config_string = r#"
        [
            {
                "name": "function",
                "language": "rust",
                "key_definitions": [
                    { "key": "function_item", "depth": 1 },
                    { "key": "identifier", "depth": 2 }
                ]
            }
        ]
"#;
        let settings = HighlightSettings::load_settings(&[config_string]);
        let definitions = settings.definitions;
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "function");
        assert_eq!(definitions[0].language, "rust");
        assert_eq!(definitions[0].key_definitions.len(), 2);
    }
}
