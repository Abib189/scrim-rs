//! Fix Z2: an agent's reply is drawn from its Markdown subset, with no marker left as text; your
//! own message and the notes stay plain; nothing reaches past the panel at 240, 320 and 400 px, in
//! either theme, with Spanish text, a very long word and a long line of code.

use egui::accesskit::Role;
use serde_json::Value;

use crate::follow_agent::chat_ui::Line;
use crate::follow_agent::chat_ui::tests::window;
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::tree;

const REPLY: &str = "Here are **two directions** for the poster:

# Swiss
- **1. Swiss: \"Poster — Form\"** a grid of *six* columns
* `Inter`, 96 pt, as in [the guide](https://example.com/guide)

1. Rojo sobre blanco: una composición extraordinariamente equilibrada con márgenes amplios
2. Supercalifragilisticexpialidocious_anticonstitucionalmente_desproporcionadamente

```
fn main() { println!(\"a very long line of code that wraps rather than scrolling sideways\"); }
```
<b>not html</b> and 2 ** 3";

const MINE: &str = "Keep **this** and `that` as I typed them";

fn replies(u: &Ui) -> Vec<Value> {
    crate::follow_agent::inspect(&u.app)["aiPanel"]["replies"].as_array().cloned().unwrap_or_default()
}

fn with_reply(dark: bool, w: f32) -> Ui {
    let (mut u, _) = window();
    u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
    u.app.ui.right_width = w;
    u.app.ui.agent_view.chat.lines = vec![
        Line::You(MINE.into()),
        Line::Reply { agent: "Claude Code".into(), text: REPLY.into() },
        Line::Note("A note with **stars** stays as written".into()),
    ];
    u.frame(vec![]);
    u.frame(vec![]);
    u
}

#[test]
fn a_reply_is_drawn_as_its_markdown_with_no_marker_left() {
    let mut u = with_reply(false, 336.0);
    let r = replies(&u);
    assert_eq!(r.len(), 1, "one reply: {r:?}");
    assert_eq!(r[0]["agent"], "Claude Code");
    let blocks = r[0]["blocks"].as_array().unwrap();
    let kinds: Vec<&str> = blocks.iter().map(|b| b["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["paragraph", "heading", "bullet", "bullet", "numbered", "numbered", "code", "paragraph"]);
    for b in blocks.iter().filter(|b| b["kind"] != "code") {
        let t = b["text"].as_str().unwrap();
        assert!(!t.contains("**") || t.ends_with("2 ** 3"), "no literal ** left: {t}");
        assert!(!t.contains('`') && !t.contains("](") && !t.starts_with('#') && !t.starts_with("- "), "no marker left: {t}");
    }
    assert_eq!(blocks[1]["text"], "Swiss");
    assert_eq!(blocks[2]["text"], "1. Swiss: \"Poster — Form\" a grid of six columns");
    assert_eq!(blocks[3]["text"], "Inter, 96 pt, as in the guide", "a link is its text: its address isn't shown");
    assert_eq!(blocks[7]["text"], "<b>not html</b> and 2 ** 3", "raw HTML and an unmatched ** stay as written");
    // Your message and the note are plain text, markers and all.
    let nodes = tree(&mut u);
    for plain in [MINE, "A note with **stars** stays as written"] {
        assert!(nodes.iter().any(|n| n.label() == Some(plain) || n.value() == Some(plain)), "{plain} shown as typed");
    }
}

#[test]
fn a_reply_never_reaches_past_the_panel_at_240_320_and_400() {
    for dark in [false, true] {
        for w in [240.0, 320.0, 400.0] {
            let mut u = with_reply(dark, w);
            let p = crate::follow_agent::inspect(&u.app)["aiPanel"].clone();
            assert!(p["widest"].as_f64().unwrap() <= p["width"].as_f64().unwrap() + 1.0, "no sideways scroll ({dark} {w})");
            let nodes = tree(&mut u);
            let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
            let blocks = replies(&u)[0]["blocks"].as_array().cloned().unwrap();
            assert_eq!(blocks.len(), 8, "{dark} {w}");
            for b in &blocks {
                let r: Vec<f64> = b["rect"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
                assert!(r[0] >= pane.x0 + 15.5 && r[2] <= pane.x1 - 15.5, "{dark} {w}: {} 16 inside: {r:?} in {pane:?}", b["kind"]);
            }
            for n in nodes.iter().filter(|n| n.role() == Role::Label && n.bounds().is_some_and(|b| b.x0 >= pane.x0 && b.y0 >= pane.y0)) {
                let b = n.bounds().unwrap();
                assert!(b.x1 <= pane.x1 + 0.5, "{dark} {w}: {:?} past the panel: {b:?} in {pane:?}", n.label().or(n.value()));
            }
        }
    }
}
