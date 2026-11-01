#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heading {
    H1(String),
    H2(String),
    H3(String),
    H4(String),
    H5(String),
    H6(String),
}

impl Heading {
    fn new(level: usize, title: String) -> Self {
        match level {
            1 => Self::H1(title),
            2 => Self::H2(title),
            3 => Self::H3(title),
            4 => Self::H4(title),
            5 => Self::H5(title),
            _ => Self::H6(title),
        }
    }

    pub fn level(&self) -> usize {
        match self {
            Heading::H1(_) => 1,
            Heading::H2(_) => 2,
            Heading::H3(_) => 3,
            Heading::H4(_) => 4,
            Heading::H5(_) => 5,
            Heading::H6(_) => 6,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Heading::H1(title)
            | Heading::H2(title)
            | Heading::H3(title)
            | Heading::H4(title)
            | Heading::H5(title)
            | Heading::H6(title) => title,
        }
    }
}

#[derive(Debug, Clone)]
struct HeadingNode {
    start_byte: usize,
    end_byte: usize,
    heading: Heading,
}

pub fn split_headings(markdown: &str) -> Vec<(Heading, String)> {
    let mut parser = arborium::tree_sitter::Parser::new();
    parser
        .set_language(&arborium::lang_markdown::language().into())
        .expect("failed to set markdown language");

    let Some(tree) = parser.parse(markdown, None) else {
        return Vec::new();
    };

    let mut nodes = Vec::new();
    collect_heading_nodes(markdown, tree.root_node(), &mut nodes);
    nodes.sort_by_key(|node| node.start_byte);

    let mut sections = Vec::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        let section_end = nodes
            .get(i + 1)
            .map(|next| next.start_byte)
            .unwrap_or(markdown.len());

        let body = markdown
            .get(node.end_byte..section_end)
            .unwrap_or("")
            .trim()
            .to_string();

        sections.push((node.heading.clone(), body));
    }

    sections
}

pub fn sprint_headings(markdown: &str) -> Vec<(Heading, String)> {
    split_headings(markdown)
}

/// 文書が見出しで始まっている場合、その見出しを返す。
/// 見出しより前に空白以外の内容がある場合は None を返す。
pub fn leading_heading(markdown: &str) -> Option<Heading> {
    let mut parser = arborium::tree_sitter::Parser::new();
    parser
        .set_language(&arborium::lang_markdown::language().into())
        .expect("failed to set markdown language");
    let tree = parser.parse(markdown, None)?;

    let mut nodes = Vec::new();
    collect_heading_nodes(markdown, tree.root_node(), &mut nodes);
    let first = nodes.into_iter().min_by_key(|node| node.start_byte)?;
    markdown
        .get(..first.start_byte)?
        .trim()
        .is_empty()
        .then_some(first.heading)
}

fn collect_heading_nodes(
    markdown: &str,
    node: arborium::tree_sitter::Node<'_>,
    out: &mut Vec<HeadingNode>,
) {
    let kind = node.kind();

    if (kind == "atx_heading" || kind == "setext_heading")
        && let Some((level, title)) = parse_heading(markdown, node)
    {
        out.push(HeadingNode {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            heading: Heading::new(level, title),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_heading_nodes(markdown, child, out);
    }
}

fn parse_heading(markdown: &str, node: arborium::tree_sitter::Node<'_>) -> Option<(usize, String)> {
    let text = node.utf8_text(markdown.as_bytes()).ok()?;

    match node.kind() {
        "atx_heading" => parse_atx_heading(text),
        "setext_heading" => parse_setext_heading(text),
        _ => None,
    }
}

fn parse_atx_heading(text: &str) -> Option<(usize, String)> {
    let line = text.lines().next()?.trim();
    let level = line.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }

    let rest = line[level..].trim();
    // CommonMark では閉じの # 列は直前に空白があるときだけ取り除く (`# C#` は `C#` のまま)
    let without_closing = rest.trim_end_matches('#');
    let title = if without_closing.is_empty() || without_closing.ends_with([' ', '\t']) {
        without_closing.trim_end()
    } else {
        rest
    };
    Some((level, title.to_string()))
}

fn parse_setext_heading(text: &str) -> Option<(usize, String)> {
    let mut lines = text.lines();
    let title = lines.next()?.trim().to_string();
    let underline = lines.next()?.trim();

    let level = if underline.starts_with('=') {
        1
    } else if underline.starts_with('-') {
        2
    } else {
        return None;
    };

    Some((level, title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_atx_headings() {
        let markdown = "# H1\nfirst\n\n## H2\nsecond\n";
        let sections = split_headings(markdown);

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, Heading::H1("H1".to_string()));
        assert_eq!(sections[0].1, "first".to_string());
        assert_eq!(sections[1].0, Heading::H2("H2".to_string()));
        assert_eq!(sections[1].1, "second".to_string());
    }

    #[test]
    fn split_setext_headings() {
        let markdown = "Title\n=====\nA\n\nSub\n---\nB\n";
        let sections = split_headings(markdown);

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, Heading::H1("Title".to_string()));
        assert_eq!(sections[0].1, "A".to_string());
        assert_eq!(sections[1].0, Heading::H2("Sub".to_string()));
        assert_eq!(sections[1].1, "B".to_string());
    }

    #[test]
    fn sprint_headings_is_alias() {
        let markdown = "### X\ntext\n";
        assert_eq!(sprint_headings(markdown), split_headings(markdown));
    }

    #[test]
    fn closing_sequence_requires_preceding_space() {
        assert_eq!(parse_atx_heading("# C#"), Some((1, "C#".to_string())));
        assert_eq!(
            parse_atx_heading("# 見出し ##"),
            Some((1, "見出し".to_string()))
        );
        assert_eq!(parse_atx_heading("# ###"), Some((1, String::new())));
    }

    #[test]
    fn leading_heading_only_when_document_starts_with_it() {
        let cases = [
            ("# 買い物\n牛乳", Some(Heading::H1("買い物".to_string()))),
            (
                "\n\n  # 前後の空白  \n",
                Some(Heading::H1("前後の空白".to_string())),
            ),
            (
                "タイトル\n=====\n本文",
                Some(Heading::H1("タイトル".to_string())),
            ),
            ("## 見出し2", Some(Heading::H2("見出し2".to_string()))),
            ("#見出しではない", None),
            ("    # コードブロック", None),
            ("> # 引用", None),
            ("本文\n\n# 2 つ目の段落", None),
            ("", None),
        ];
        for (markdown, expected) in cases {
            assert_eq!(leading_heading(markdown), expected, "{markdown:?}");
        }
    }
}
