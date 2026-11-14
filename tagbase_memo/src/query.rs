use crate::{Error, Result};

#[derive(Debug, Default)]
pub(crate) struct Query {
    pub(crate) keywords: Vec<String>,
    pub(crate) tags: Vec<String>,
}

impl Query {
    pub(crate) fn parse(input: &str) -> Result<Self> {
        let mut query = Self::default();
        let mut remaining = input.trim();
        while !remaining.is_empty() {
            let is_tag = remaining.starts_with("tag:");
            if is_tag {
                remaining = &remaining[4..];
            }
            let mut value = String::new();
            if let Some(quoted) = remaining.strip_prefix('"') {
                let mut chars = quoted.char_indices();
                let mut end = None;
                while let Some((index, character)) = chars.next() {
                    match character {
                        '"' => {
                            end = Some(index + 1);
                            break;
                        }
                        '\\' => match chars.next() {
                            Some((_, escaped @ ('"' | '\\'))) => value.push(escaped),
                            _ => return Err(Error::InvalidQuery(input.to_owned())),
                        },
                        _ => value.push(character),
                    }
                }
                let end = end.ok_or_else(|| Error::InvalidQuery(input.to_owned()))?;
                remaining = &quoted[end..];
                if remaining.starts_with(|c: char| !c.is_whitespace()) {
                    return Err(Error::InvalidQuery(input.to_owned()));
                }
            } else {
                let end = remaining
                    .find(char::is_whitespace)
                    .unwrap_or(remaining.len());
                value.push_str(&remaining[..end]);
                remaining = &remaining[end..];
                if value.contains('"') {
                    return Err(Error::InvalidQuery(input.to_owned()));
                }
            }
            if value.is_empty() {
                return Err(Error::InvalidQuery(input.to_owned()));
            }
            if is_tag {
                query.tags.push(value);
            } else {
                query.keywords.push(value.to_lowercase());
            }
            remaining = remaining.trim_start();
        }
        Ok(query)
    }
}
