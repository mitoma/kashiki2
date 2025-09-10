fn main() {
    let target_string = r#"
# Hello, world!
This is a **bold** text and *italic* text.

## morning show

[super link](https://example.com)

---

- [x] Task 1
- [ ] Task 2
1. First item
2. Second item
> This is a blockquote.
Here is some `inline code`.

```rust
fn main() {
    println!("Hello, world!");
}
```
"#;

    //let target_string = "# title\n\nInline [content].\n";

    let mut md_parser = tree_sitter_md::MarkdownParser::default();

    let Some(result) = md_parser.parse(target_string.as_bytes(), None) else {
        panic!("failed to parse");
    };

    let mut cursor = result.walk();
    println!("{:?}", cursor.field_name());
    let mut skip_child = false;
    let mut skip_print = false;
    let mut indent = 0;
    loop {
        if !skip_print {
            println!(
                "{} {:?} {:?} {:?}",
                "  ".repeat(indent as usize),
                cursor.node().kind(),
                cursor.node().start_position(),
                cursor.node().end_position()
            );
        }
        if !skip_child && cursor.goto_first_child() {
            //println!("child");
            indent += 1;
            skip_child = false;
        } else if cursor.goto_next_sibling() {
            //println!("next");
            skip_child = false;
            skip_print = false;
        } else {
            //println!("goto parent");
            if !cursor.goto_parent() {
                break;
            }
            indent -= 1;
            skip_child = true;
            skip_print = true;
        }
    }
}
