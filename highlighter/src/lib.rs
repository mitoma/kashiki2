pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

fn walk(target_string: &str, cursor: &mut tree_sitter::TreeCursor, indent: usize, in_inline: bool) {
    println!(
        "{}{:?} {:?} {:?}",
        "  ".repeat(indent),
        cursor.node().kind(),
        cursor.node().start_position(),
        cursor.node().end_position()
    );

    let mut require_children = true;
    match cursor.node().kind() {
        "inline" if !in_inline => {
            let mut parser = md_inline_parser();
            let tree = parser
                .parse(
                    &target_string[cursor.node().start_byte()..cursor.node().end_byte()],
                    None,
                )
                .unwrap();
            let mut inner_cursor = tree.root_node().walk();
            walk(target_string, &mut inner_cursor, indent + 1, true);
            require_children = false;
        }
        "code_fence_content" => {
            let mut parser = rust_parser();
            let tree = parser
                .parse(
                    &target_string[cursor.node().start_byte()..cursor.node().end_byte()],
                    None,
                )
                .unwrap();
            let mut inner_cursor = tree.root_node().walk();
            walk(target_string, &mut inner_cursor, indent + 1, true);
            require_children = false;
        }
        _ => {}
    }

    if require_children && cursor.goto_first_child() {
        walk(target_string, cursor, indent + 1, in_inline);
        cursor.goto_parent();
    }
    if cursor.goto_next_sibling() {
        walk(target_string, cursor, indent, in_inline);
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

goodbye!
"#;

        let mut parser = md_parser();
        let tree = parser.parse(target_string, None).unwrap();
        let cursor = tree.root_node().walk();
        walk(target_string, &mut cursor.clone(), 0, false);
    }
}
