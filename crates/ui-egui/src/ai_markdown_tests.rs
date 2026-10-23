//! Fix Z2: the reply's Markdown subset.

use super::{Block, Span, inline, parse};

fn text(s: &str) -> Span {
    Span::Text(s.into())
}

#[test]
fn asterisks_and_bullets_become_bold_and_list_items() {
    let b = parse("**1. Swiss: \"Poster — Form\"**\n\n- A grid of *six* columns\n* `Inter`, 96 pt\n2. Red on white");
    assert_eq!(
        b,
        [
            Block::Paragraph(vec![Span::Bold("1. Swiss: \"Poster — Form\"".into())]),
            Block::Bullet(vec![text("A grid of "), Span::Italic("six".into()), text(" columns")]),
            Block::Bullet(vec![Span::Code("Inter".into()), text(", 96 pt")]),
            Block::Numbered("2.".into(), vec![text("Red on white")]),
        ]
    );
    assert!(b.iter().all(|b| !b.text().contains("**")), "no literal asterisks left");
}

#[test]
fn an_unmatched_marker_stays_as_written() {
    assert_eq!(inline("2 ** 3 is 8"), [text("2 ** 3 is 8")]);
    assert_eq!(inline("**bold** and ** alone"), [Span::Bold("bold".into()), text(" and ** alone")]);
    assert_eq!(inline("a * b * c"), [text("a * b * c")], "spaced stars aren't italics");
    assert_eq!(inline("`open"), [text("`open")]);
    assert_eq!(inline("<b>html</b> stays text"), [text("<b>html</b> stays text")]);
}

#[test]
fn code_fences_keep_their_lines() {
    let b = parse("Before\n```rust\nlet x = **1**;\n  indented\n```\nAfter");
    assert_eq!(b, [Block::Paragraph(vec![text("Before")]), Block::Code("let x = **1**;\n  indented".into()), Block::Paragraph(vec![text("After")])]);
    // Unclosed: to the end.
    assert_eq!(parse("```\na\nb"), [Block::Code("a\nb".into())]);
}

#[test]
fn bold_nested_in_a_bullet_and_headings_and_links() {
    let b = parse("# Two posters\n- **Swiss** grid, see [the guide](https://example.com/x)\nA line\njoined");
    assert_eq!(
        b,
        [
            Block::Heading(vec![text("Two posters")]),
            Block::Bullet(vec![Span::Bold("Swiss".into()), text(" grid, see "), Span::Link("the guide".into())]),
            Block::Paragraph(vec![text("A line joined")]),
        ]
    );
    assert!(!b.iter().any(|b| b.text().contains("https")), "a link's address isn't shown");
}

#[test]
fn empty_input_and_very_long_lines() {
    assert!(parse("").is_empty());
    assert!(parse("\n\n  \n").is_empty());
    let long = "word ".repeat(400) + &"x".repeat(300);
    let b = parse(&long);
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].text(), long.trim());
}
