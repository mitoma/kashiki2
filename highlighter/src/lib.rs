pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[derive(Clone, Debug)]
struct HighlightContext {
    target_string: String,
    in_inline: bool,
    depth: usize,
    kind_stack: Vec<String>,
    language_suggestion: Option<String>,
}

impl HighlightContext {
    fn new(target_string: &str) -> Self {
        Self {
            target_string: target_string.to_string(),
            in_inline: false,
            depth: 0,
            kind_stack: vec![],
            language_suggestion: None,
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
        new_context.language_suggestion = Some(lang.to_string());
        new_context
    }

    fn without_language_suggestion(&self) -> Self {
        let mut new_context = self.clone();
        new_context.language_suggestion = None;
        new_context
    }
}

fn walk(context: HighlightContext, cursor: &mut tree_sitter::TreeCursor) {
    let mut context = context.clone();

    let current_node = cursor.node();
    println!("{}{:?}", "  ".repeat(context.depth), context.kind_stack);
    println!(
        "{}{:?} {:?} {:?}",
        "  ".repeat(context.depth),
        current_node.kind(),
        current_node.start_position(),
        current_node.end_position()
    );

    let mut require_children = true;
    match current_node.kind() {
        "inline" if !context.in_inline => {
            let mut parser = md_inline_parser();
            let tree = parser
                .parse(
                    &context.target_string[cursor.node().start_byte()..cursor.node().end_byte()],
                    None,
                )
                .unwrap();
            let mut inner_cursor = tree.root_node().walk();
            walk(context.with_kind(current_node.kind()), &mut inner_cursor);
            require_children = false;
        }
        "code_fence_content" => {
            let mut parser = match context.language_suggestion.as_deref() {
                Some("rust") => rust_parser(),
                Some("java") => java_parser(),
                Some("go") => go_parser(),
                _ => {
                    return;
                }
            };
            let tree = parser
                .parse(
                    &context.target_string[cursor.node().start_byte()..cursor.node().end_byte()],
                    None,
                )
                .unwrap();
            let mut inner_cursor = tree.root_node().walk();
            walk(context.with_kind(current_node.kind()), &mut inner_cursor);
            require_children = false;
        }
        "info_string" => {
            let language_node = current_node.child(0).unwrap();
            let lang = &context.target_string[language_node.start_byte()..language_node.end_byte()]
                .to_string();
            context = context.with_language_suggestion(lang);
            require_children = false;
        }
        _ => {}
    }

    if require_children && cursor.goto_first_child() {
        walk(context.with_kind(current_node.kind()), cursor);
        cursor.goto_parent();
    }
    if cursor.goto_next_sibling() {
        //println!("language_suggestion: {:?}", context.language_suggestion);
        walk(context, cursor);
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

        let mut parser = md_parser();
        let tree = parser.parse(target_string, None).unwrap();
        let cursor = tree.root_node().walk();
        let context = HighlightContext::new(target_string);
        walk(context, &mut cursor.clone());
    }
}
