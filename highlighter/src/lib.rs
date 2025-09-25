use std::{ops::Range, sync::mpsc};

use crate::settings::HighlightSettings;

pub mod settings;

#[derive(Debug, Clone)]
struct CallbackArguments {
    pub language: String,
    pub kind_stack: KindStack,
}

#[derive(Debug, Clone, Default)]
struct KindStack {
    kinds: Vec<KindAndRange>,
    path: String,
}

impl KindStack {
    fn push(&mut self, kind: KindAndRange) {
        self.kinds.push(kind);
        self.path = self
            .kinds
            .iter()
            .map(|k| k.kind.clone())
            .collect::<Vec<String>>()
            .join(".");
    }

    pub fn ends_with(&self, suffix: &str) -> bool {
        self.path.ends_with(&format!(".{}", suffix))
    }

    pub fn range(&self, depth: usize) -> Range<usize> {
        self.kinds
            .len()
            .checked_sub(1 + depth)
            .map(|i| self.kinds[i].range.clone())
            .unwrap_or(0..0)
    }
}

// バイト位置から文字位置に変換するヘルパー関数
fn byte_to_char_position(text: &str, byte_pos: usize) -> usize {
    text.char_indices()
        .take_while(|(i, _)| *i < byte_pos)
        .count()
}

fn markdown_highlight_callback(target_string: &str, callback: impl Fn(CallbackArguments)) {
    let mut parser = md_parser();
    let tree = parser.parse(target_string, None).unwrap();
    let cursor = tree.root_node().walk();
    let context = HighlightContext::new(target_string);
    walk(context, &mut cursor.clone(), &callback);
}

pub fn markdown_highlight(
    target_string: &str,
    settings: &HighlightSettings,
) -> Vec<(String, Range<usize>)> {
    let (tx, rx) = mpsc::channel();
    markdown_highlight_callback(target_string, |args| {
        tx.send(args).unwrap();
    });
    // rx 側でイテレーションを終了させるためにtxをドロップする
    drop(tx);
    rx.iter()
        .filter_map(|arg| {
            println!("arg: {:?}", arg.kind_stack.path);
            settings.args_to_definition(&arg)
        })
        .collect()
}

#[derive(Clone, Debug)]
struct KindAndRange {
    kind: String,
    range: Range<usize>,
}

impl KindAndRange {
    fn new(context: &HighlightContext, node: &tree_sitter::Node) -> Self {
        Self {
            kind: node.kind().to_string(),
            range: byte_to_char_position(
                context.target_string,
                context.target_string_byte_offset + node.start_byte(),
            )
                ..byte_to_char_position(
                    context.target_string,
                    context.target_string_byte_offset + node.end_byte(),
                ),
        }
    }
}

#[derive(Clone, Debug)]
struct HighlightContext<'a> {
    target_string: &'a str,
    target_string_byte_offset: usize,
    in_inline: bool,
    depth: usize,
    kind_stack: KindStack,
    language_suggestion: String,
}

impl<'a> HighlightContext<'a> {
    fn new(target_string: &'a str) -> Self {
        Self {
            target_string,
            target_string_byte_offset: 0,
            in_inline: false,
            depth: 0,
            kind_stack: KindStack::default(),
            language_suggestion: "markdown".to_string(),
        }
    }

    fn with_kind(&self, kind: &KindAndRange) -> Self {
        let mut new_context = self.clone();
        new_context.kind_stack.push(kind.clone());
        new_context.depth += 1;
        if kind.kind == "inline" {
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

/// TreeCursorを使って深さ優先探索でノードを走査するイテレーター
pub struct TreeCursorIterator<'a> {
    cursor: tree_sitter::TreeCursor<'a>,
    first_iteration: bool,
}

impl<'a> TreeCursorIterator<'a> {
    pub fn new(cursor: tree_sitter::TreeCursor<'a>) -> Self {
        Self {
            cursor,
            first_iteration: true,
        }
    }
}

impl<'a> Iterator for TreeCursorIterator<'a> {
    type Item = tree_sitter::Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // 初回は現在位置のノードを返す
        if self.first_iteration {
            self.first_iteration = false;
            return Some(self.cursor.node());
        }

        // 子ノードがあれば下降
        if self.cursor.goto_first_child() {
            return Some(self.cursor.node());
        }

        // 兄弟ノードがあれば移動
        if self.cursor.goto_next_sibling() {
            return Some(self.cursor.node());
        }

        // 親に戻りながら兄弟ノードを探す
        while self.cursor.goto_parent() {
            if self.cursor.goto_next_sibling() {
                return Some(self.cursor.node());
            }
        }

        // すべてのノードを走査完了
        None
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
            current_stack.push(KindAndRange::new(&context, &current_node));

            callback(CallbackArguments {
                language,
                kind_stack: current_stack.clone(),
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
                        .with_kind(&KindAndRange::new(&context, &current_node))
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
                    "json" => json_parser(),
                    "bash" => bash_parser(),
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
                        .with_kind(&KindAndRange::new(&context, &current_node))
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
            walk(
                context.with_kind(&KindAndRange::new(&context, &current_node)),
                cursor,
                callback,
            );
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

fn json_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_json::LANGUAGE.into())
        .unwrap();
    parser
}

fn bash_parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .unwrap();
    parser
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::settings::HighlightSettings;

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

        markdown_highlight_callback(
            target_string,
            |CallbackArguments {
                 language,
                 kind_stack,
             }| {
                let indent = "  ".repeat(kind_stack.kinds.len());
                println!("{}-----", indent);
                println!("{}lang: \"{}\"", indent, language);
                println!("{}Kind stack: {}", indent, kind_stack.path);
                println!("{}Range: {:?}", indent, kind_stack.range(0));
                if language == "rust" && kind_stack.ends_with("function_item.identifier") {
                    println!(
                        "{}Matched text: {}",
                        indent,
                        target_string.chars().collect::<Vec<_>>()[kind_stack.range(0)]
                            .iter()
                            .collect::<String>()
                    );
                }
            },
        );
    }

    #[test]
    fn test_utf8() {
        let target_string = "やさしい🐖**健康料理**365日";
        let has_strong = Mutex::new(false);
        markdown_highlight_callback(target_string, |CallbackArguments { kind_stack, .. }| {
            if kind_stack.ends_with("strong_emphasis") {
                *has_strong.lock().unwrap() = true;
                assert_eq!(
                    "**健康料理**",
                    target_string.chars().collect::<Vec<_>>()[kind_stack.range(0)]
                        .iter()
                        .collect::<String>()
                );
            }
        });
        assert!(*has_strong.lock().unwrap());
    }

    #[test]
    fn test_iter() {
        let target_string = r#"
# Hello, world!

This is a **bold** text.
"#;

        let mut parser = md_parser();
        let tree = parser.parse(target_string, None).unwrap();
        let cursor = tree.root_node().walk();

        let iter = TreeCursorIterator::new(cursor);
        let nodes: Vec<_> = iter.collect();

        // ルートノードから開始して、深さ優先で走査されることを確認
        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].kind(), "document");

        // 各ノードが適切に走査されることを確認
        for node in &nodes {
            println!(
                "Node kind: {}, text: {:?}",
                node.kind(),
                &target_string[node.start_byte()..node.end_byte()]
            );
        }
    }

    #[test]
    fn test_highlight_settings() {
        let settings = HighlightSettings::default();

        let target_string = r#"
```go
func main() {
    var s = []string{"foo", "bar", "zoo"}
}
```
"#;

        let result = markdown_highlight(target_string, &settings);
        println!("result: {:?}", result);
        let categories = {
            let mut c = settings.categories();
            c.sort();
            c
        };
        println!("categories: {:?}", categories);
    }
}
