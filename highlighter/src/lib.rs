#[derive(Debug, Clone)]
pub struct CallbackArguments {
    pub language: String,
    pub kind_stack: Vec<String>,
    pub start: usize,
    pub end: usize,
}

// バイト位置から文字位置に変換するヘルパー関数
fn byte_to_char_position(text: &str, byte_pos: usize) -> usize {
    text.char_indices()
        .take_while(|(i, _)| *i < byte_pos)
        .count()
}

pub fn markdown_highlight(target_string: &str, callback: impl Fn(CallbackArguments)) {
    let mut parser = md_parser();
    let tree = parser.parse(target_string, None).unwrap();
    let cursor = tree.root_node().walk();
    let context = HighlightContext::new(target_string);
    walk(context, &mut cursor.clone(), &callback);
}

#[derive(Clone, Debug)]
struct HighlightContext<'a> {
    target_string: &'a str,
    target_string_byte_offset: usize,
    in_inline: bool,
    depth: usize,
    kind_stack: Vec<String>,
    language_suggestion: String,
}

impl<'a> HighlightContext<'a> {
    fn new(target_string: &'a str) -> Self {
        Self {
            target_string,
            target_string_byte_offset: 0,
            in_inline: false,
            depth: 0,
            kind_stack: vec![],
            language_suggestion: "markdown".to_string(),
        }
    }

    fn with_kind(&self, kind: &str) -> Self {
        let mut new_context = self.clone();
        new_context.kind_stack.push(kind.to_string());
        new_context.depth += 1;
        if kind == "inline" {
            new_context.in_inline = true;
        }
        new_context
    }

    fn with_language_suggestion(&self, lang: &str) -> Self {
        let mut new_context = self.clone();
        new_context.language_suggestion = lang.to_string();
        new_context
    }

    fn with_byte_offset(&self, byte_offset: usize) -> Self {
        let mut new_context = self.clone();
        new_context.target_string_byte_offset += byte_offset;
        new_context
    }
}

fn walk<'a>(
    context: HighlightContext<'a>,
    cursor: &mut tree_sitter::TreeCursor,
    callback: &impl Fn(CallbackArguments),
) {
    let mut context = context.clone();

    loop {
        let current_node = cursor.node();
        {
            let language = context.language_suggestion.clone();
            let mut current_stack = context.kind_stack.clone();
            current_stack.push(current_node.kind().to_string());

            let start_byte = current_node.start_byte() + context.target_string_byte_offset;
            let end_byte = current_node.end_byte() + context.target_string_byte_offset;

            callback(CallbackArguments {
                language,
                kind_stack: current_stack,
                start: byte_to_char_position(context.target_string, start_byte),
                end: byte_to_char_position(context.target_string, end_byte),
            });
        }

        let mut require_children = true;
        match current_node.kind() {
            "inline" if !context.in_inline => {
                let mut parser = md_inline_parser();
                let tree = parser
                    .parse(
                        &context.target_string
                            [cursor.node().start_byte()..cursor.node().end_byte()],
                        None,
                    )
                    .unwrap();
                let mut inner_cursor = tree.root_node().walk();
                walk(
                    context
                        .with_kind(current_node.kind())
                        .with_byte_offset(cursor.node().start_byte()),
                    &mut inner_cursor,
                    callback,
                );
                require_children = false;
            }
            "code_fence_content" => {
                let mut parser = match context.language_suggestion.as_str() {
                    "rust" => rust_parser(),
                    "java" => java_parser(),
                    "go" => go_parser(),
                    _ => {
                        return;
                    }
                };
                let tree = parser
                    .parse(
                        &context.target_string
                            [cursor.node().start_byte()..cursor.node().end_byte()],
                        None,
                    )
                    .unwrap();
                let mut inner_cursor = tree.root_node().walk();
                walk(
                    context
                        .with_kind(current_node.kind())
                        .with_byte_offset(cursor.node().start_byte()),
                    &mut inner_cursor,
                    callback,
                );
                require_children = false;
            }
            "info_string" => {
                let language_node = current_node.child(0).unwrap();
                let lang = &context.target_string
                    [language_node.start_byte()..language_node.end_byte()]
                    .to_string();
                context = context.with_language_suggestion(lang);
                require_children = false;
            }
            _ => {}
        }

        if require_children && cursor.goto_first_child() {
            walk(context.with_kind(current_node.kind()), cursor, callback);
            cursor.goto_parent();
        }
        if !cursor.goto_next_sibling() {
            // 次の要素が無ければ抜ける
            break;
        }
    }
}

fn md_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_md::LANGUAGE.into())
        .unwrap();
    parser
}

fn md_inline_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_md::INLINE_LANGUAGE.into())
        .unwrap();
    parser
}

fn rust_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    parser
}

fn java_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .unwrap();
    parser
}

fn go_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .unwrap();
    parser
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sitter() {
        let target_string = r#"
# Hello, world!

This is a **bold** text and *italic* text.

## Hoge, world2

[super link](https://example.com)

- Indent 1
- Indent 2
    - Indent 3

- [ ] Task 1
- [x] Task 2

```rust
fn main() {
    let mut x = 1 + 2 * (3 / 4);
    test_add();
    println!("Hello, world!");
}


    /// comment
    /// fn main() {
    ///     let mut x = 1 + 2 * (3 / 4);
    ///     test_add();
    ///     println!("Hello, world!");
    /// }
    fn test_add() {
        println!("add!");
    }
```

Go Java

```java
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
```

goodbye!
"#;

        markdown_highlight(
            target_string,
            |CallbackArguments {
                 language,
                 kind_stack,
                 start,
                 end,
             }| {
                let indent = "  ".repeat(kind_stack.len());
                println!("{}-----", indent);
                println!("{}lang: \"{}\"", indent, language);
                println!("{}Kind stack: {:?}", indent, kind_stack.join("."));
                println!("{}Start: {}, End: {}", indent, start, end);
                if language == "rust"
                    && kind_stack.ends_with(&["function_item".into(), "identifier".into()])
                {
                    println!(
                        "{}Matched text: {}",
                        indent,
                        target_string
                            .chars()
                            .skip(start)
                            .take(end - start)
                            .collect::<String>()
                    );
                }
            },
        );
    }
}
