//! The agent's replies, as a small safe subset of Markdown (Fix Z2, `docs/design/ai-panel.md`):
//! **bold**, *italic*, `inline code`, "- " / "* " bullets and "1." numbered lists, paragraphs,
//! `#` headings, fenced code blocks, and links shown as their text (nothing is opened or fetched).
//! Anything else is plain text: raw HTML is never interpreted, and a marker without its pair
//! (an unmatched `**`) stays as written. Pure: no egui here; the panel draws the blocks.

/// A run of text in one style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    /// A link's text (its address is dropped: links open nothing here).
    Link(String),
}

/// A block of the reply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Span>),
    Heading(Vec<Span>),
    Bullet(Vec<Span>),
    /// The item's number as written ("1."), and its text.
    Numbered(String, Vec<Span>),
    /// A fenced code block's lines, as written.
    Code(String),
}

impl Block {
    /// Its kind, for `ui.inspect`.
    pub fn kind(&self) -> &'static str {
        match self {
            Block::Paragraph(_) => "paragraph",
            Block::Heading(_) => "heading",
            Block::Bullet(_) => "bullet",
            Block::Numbered(..) => "numbered",
            Block::Code(_) => "code",
        }
    }

    /// Its text as shown (markers gone, styles dropped).
    pub fn text(&self) -> String {
        match self {
            Block::Code(c) => c.clone(),
            Block::Paragraph(s) | Block::Heading(s) | Block::Bullet(s) | Block::Numbered(_, s) => spans_text(s),
        }
    }
}

/// The spans' text, styles dropped.
pub fn spans_text(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|s| match s {
            Span::Text(t) | Span::Bold(t) | Span::Italic(t) | Span::Code(t) | Span::Link(t) => t.as_str(),
        })
        .collect()
}

/// `src` as blocks.
pub fn parse(src: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, out: &mut Vec<Block>| {
        if !para.is_empty() {
            out.push(Block::Paragraph(inline(&para.join(" "))));
            para.clear();
        }
    };
    let mut lines = src.lines().peekable();
    while let Some(line) = lines.next() {
        let t = line.trim();
        if t.starts_with("```") {
            flush(&mut para, &mut out);
            let mut code = Vec::new();
            for l in lines.by_ref() {
                if l.trim().starts_with("```") {
                    break;
                }
                code.push(l);
            }
            out.push(Block::Code(code.join("\n")));
        } else if t.is_empty() {
            flush(&mut para, &mut out);
        } else if let Some(h) = heading(t) {
            flush(&mut para, &mut out);
            out.push(Block::Heading(inline(h)));
        } else if let Some(item) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            flush(&mut para, &mut out);
            out.push(Block::Bullet(inline(item.trim())));
        } else if let Some((n, item)) = numbered(t) {
            flush(&mut para, &mut out);
            out.push(Block::Numbered(n, inline(item.trim())));
        } else {
            para.push(t.to_string());
        }
    }
    flush(&mut para, &mut out);
    out
}

/// "# Title" … "###### Title": the title.
fn heading(t: &str) -> Option<&str> {
    let hashes = t.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&hashes).then(|| t.get(hashes..)).flatten().and_then(|rest| rest.strip_prefix(' ')).map(str::trim)
}

/// "12. Item": ("12.", "Item").
fn numbered(t: &str) -> Option<(String, &str)> {
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 4 {
        return None;
    }
    let rest = t.get(digits..)?.strip_prefix(". ")?;
    Some((format!("{}.", t.get(..digits)?), rest))
}

/// One line's spans: `code`, **bold**, *italic* or _italic_, [text](address). A marker without its
/// closing pair is plain text.
pub fn inline(src: &str) -> Vec<Span> {
    let chars: Vec<char> = src.chars().collect();
    let between = |a: usize, b: usize| chars.get(a..b).map(|s| s.iter().collect::<String>()).unwrap_or_default();
    let mut out: Vec<Span> = Vec::new();
    let mut text = String::new();
    let find = |from: usize, pat: &[char]| (from..chars.len()).find(|&i| chars.get(i..).is_some_and(|r| r.starts_with(pat)));
    let mut i = 0;
    let push = |out: &mut Vec<Span>, text: &mut String, span: Span| {
        if !text.is_empty() {
            out.push(Span::Text(std::mem::take(text)));
        }
        out.push(span);
    };
    while let Some(&c) = chars.get(i) {
        let rest = chars.get(i..).unwrap_or_default();
        if c == '`'
            && let Some(end) = find(i + 1, &['`'])
            && end > i + 1
        {
            push(&mut out, &mut text, Span::Code(between(i + 1, end)));
            i = end + 1;
        } else if rest.starts_with(&['*', '*'])
            && let Some(end) = find(i + 2, &['*', '*'])
            && end > i + 2
        {
            push(&mut out, &mut text, Span::Bold(between(i + 2, end)));
            i = end + 2;
        } else if (c == '*' || c == '_')
            && chars.get(i + 1).is_some_and(|n| !n.is_whitespace() && *n != c)
            && let Some(end) = find(i + 1, &[c])
            && end > i + 1
        {
            push(&mut out, &mut text, Span::Italic(between(i + 1, end)));
            i = end + 1;
        } else if c == '['
            && let Some(close) = find(i + 1, &[']'])
            && chars.get(close + 1) == Some(&'(')
            && let Some(paren) = find(close + 2, &[')'])
        {
            push(&mut out, &mut text, Span::Link(between(i + 1, close)));
            i = paren + 1;
        } else {
            text.push(c);
            i += 1;
        }
    }
    if !text.is_empty() {
        out.push(Span::Text(text));
    }
    out
}

#[cfg(test)]
#[path = "ai_markdown_tests.rs"]
mod tests;
