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
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_md::LANGUAGE.into())
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

fn collect_heading_nodes(markdown: &str, node: tree_sitter::Node<'_>, out: &mut Vec<HeadingNode>) {
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

fn parse_heading(markdown: &str, node: tree_sitter::Node<'_>) -> Option<(usize, String)> {
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
    let title = rest.trim_end_matches('#').trim().to_string();
    Some((level, title))
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
        assert_eq!(sections[0].1, "first\n\n".to_string());
        assert_eq!(sections[1].0, Heading::H2("H2".to_string()));
        assert_eq!(sections[1].1, "second\n".to_string());
    }

    #[test]
    fn split_setext_headings() {
        let markdown = "Title\n=====\nA\n\nSub\n---\nB\n";
        let sections = split_headings(markdown);

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, Heading::H1("Title".to_string()));
        assert_eq!(sections[0].1, "A\n\n".to_string());
        assert_eq!(sections[1].0, Heading::H2("Sub".to_string()));
        assert_eq!(sections[1].1, "B\n".to_string());
    }

    #[test]
    fn sprint_headings_is_alias() {
        let markdown = "### X\ntext\n";
        assert_eq!(sprint_headings(markdown), split_headings(markdown));
    }
}
