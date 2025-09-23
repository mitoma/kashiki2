use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::CallbackArguments;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HighlightCategoryDefinition {
    pub name: String,
    pub language: String,
    pub key_definitions: Vec<KeyDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyDefinition {
    pub key: String,
    pub depth: usize,
}

pub fn load_definitions(config_string: &str) -> Vec<HighlightCategoryDefinition> {
    serde_json::from_str(config_string).unwrap()
}

pub fn args_to_definition(
    definitions: Vec<HighlightCategoryDefinition>,
    arg: CallbackArguments,
) -> Option<(String, Range<usize>)> {
    for def in definitions {
        if def.language == arg.language {
            for key_def in def.key_definitions.iter() {
                if arg.kind_stack.ends_with(&key_def.key) {
                    return Some((def.name.clone(), arg.kind_stack.range(key_def.depth)));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::settings::load_definitions;

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
        let definitions = load_definitions(config_string);
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "function");
        assert_eq!(definitions[0].language, "rust");
        assert_eq!(definitions[0].key_definitions.len(), 2);
    }
}
