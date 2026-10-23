//! An agent's reply in the transcript, drawn from its Markdown subset (Fix Z2,
//! `docs/design/ai-panel.md`; the parser is `ai_markdown.rs`):
//! - **Text:** 13 px, `t.text`; **bold** and `#` headings SemiBold at the same size; *italic*;
//!   `inline code` monospace on `t.field`; a link's text in `t.accent_text` (it opens nothing).
//! - **Lists:** the bullet ("•", `t.text_secondary`) or the number, then the item with a hanging
//!   indent: its wrapped lines start under its first word.
//! - **Code blocks:** monospace in a `t.field` box, wrapped (a long word is broken), never scrolled
//!   sideways.
//!
//! Every block wraps to the width it is given, so nothing reaches past the panel. Raw HTML is text.

use egui::text::{LayoutJob, TextFormat};
use egui::{CornerRadius, FontFamily, FontId, Rect, Sense, Ui, vec2};
use serde_json::{Value, json};

use super::markdown::{Block, Span, parse};
use super::panel_theme::Roles;
use crate::theme::{italic, regular, semibold};

const SIZE: f32 = 13.0;
const LINE: f32 = SIZE * 1.4;
const CODE: f32 = 12.0;
/// Between blocks; the hanging indent's least width; a code box's padding.
const GAP: f32 = 6.0;
const INDENT: f32 = 18.0;
const PAD: f32 = 8.0;

fn mono() -> FontId {
    FontId::new(CODE, FontFamily::Monospace)
}

fn format(font: FontId, ro: &Roles) -> TextFormat {
    TextFormat { font_id: font, color: ro.text, line_height: Some(LINE), ..Default::default() }
}

/// `spans` as one wrapped job in `base`'s font, `width` wide.
fn job(spans: &[Span], base: FontId, ro: &Roles, width: f32) -> LayoutJob {
    let mut j = LayoutJob::default();
    j.wrap.max_width = width;
    for s in spans {
        let plain = format(base.clone(), ro);
        let (text, f) = match s {
            Span::Text(t) => (t, plain),
            Span::Bold(t) => (t, TextFormat { font_id: semibold(SIZE), ..plain }),
            // The real Inter Italic face (Fix AA1), not a slanted regular.
            Span::Italic(t) => (t, TextFormat { font_id: italic(SIZE), ..plain }),
            Span::Code(t) => (t, TextFormat { font_id: mono(), background: ro.code, ..plain }),
            Span::Link(t) => (t, TextFormat { color: ro.link, ..plain }),
        };
        j.append(text, 0.0, f);
    }
    j
}

/// Draws `text` across the width available; returns each block's kind, text as shown and rectangle
/// (for `ui.inspect`).
pub fn draw(ui: &mut Ui, text: &str, ro: &Roles, fonts: &mut std::collections::BTreeMap<&'static str, FontId>) -> Vec<Value> {
    let mut out = Vec::new();
    for (i, b) in parse(text).iter().enumerate() {
        note_fonts(b, fonts);
        if i > 0 {
            ui.add_space(GAP);
        }
        let w = ui.available_width();
        let r = match b {
            Block::Paragraph(s) => item(ui, None, job(s, regular(SIZE), ro, w), ro),
            Block::Heading(s) => item(ui, None, job(s, semibold(SIZE), ro, w), ro),
            Block::Bullet(s) => item(ui, Some(("•", ro.text_secondary)), job(s, regular(SIZE), ro, w - INDENT), ro),
            Block::Numbered(n, s) => {
                let indent = marker_width(ui, n, ro).max(INDENT);
                item(ui, Some((n, ro.text)), job(s, regular(SIZE), ro, w - indent), ro)
            }
            Block::Code(c) => code(ui, c, ro, w),
        };
        out.push(json!({"kind": b.kind(), "text": b.text(), "rect": [r.min.x, r.min.y, r.max.x, r.max.y]}));
    }
    out
}

/// The fonts block `b` draws with, by role (Fix AA1).
fn note_fonts(b: &Block, fonts: &mut std::collections::BTreeMap<&'static str, FontId>) {
    let spans = match b {
        Block::Code(_) => {
            fonts.insert("replyCode", mono());
            return;
        }
        Block::Heading(s) => {
            fonts.insert("replyHeading", semibold(SIZE));
            s
        }
        Block::Paragraph(s) | Block::Bullet(s) | Block::Numbered(_, s) => s,
    };
    for s in spans {
        let (role, f) = match s {
            Span::Bold(_) => ("replyBold", semibold(SIZE)),
            Span::Italic(_) => ("replyItalic", italic(SIZE)),
            Span::Code(_) => ("replyCode", mono()),
            Span::Text(_) | Span::Link(_) if !matches!(b, Block::Heading(_)) => ("reply", regular(SIZE)),
            Span::Text(_) | Span::Link(_) => continue,
        };
        fonts.insert(role, f);
    }
}

/// A number's width and the room after it.
fn marker_width(ui: &Ui, n: &str, ro: &Roles) -> f32 {
    ui.painter().layout_no_wrap(n.to_string(), regular(SIZE), ro.text).size().x + 6.0
}

/// A paragraph or a list item: the marker in the hanging indent, then the text.
fn item(ui: &mut Ui, marker: Option<(&str, egui::Color32)>, j: LayoutJob, ro: &Roles) -> Rect {
    let w = ui.available_width();
    let indent = w - j.wrap.max_width;
    let galley = ui.painter().layout_job(j);
    let (row, _) = ui.allocate_exact_size(vec2(w, galley.size().y), Sense::hover());
    if let Some((m, color)) = marker {
        let mg = ui.painter().layout_job(LayoutJob::single_section(m.to_string(), TextFormat { color, ..format(regular(SIZE), ro) }));
        ui.painter().galley(row.min, mg, color);
    }
    let r = Rect::from_min_size(row.min + vec2(indent, 0.0), galley.size());
    ui.put(r, egui::Label::new(galley));
    r
}

/// A fenced block: monospace in a `t.field` box, `PAD` inside, wrapped to fit.
fn code(ui: &mut Ui, c: &str, ro: &Roles, w: f32) -> Rect {
    let mut j = LayoutJob::single_section(c.to_string(), TextFormat { line_height: Some(CODE * 1.45), ..format(mono(), ro) });
    j.wrap.max_width = w - 2.0 * PAD;
    let galley = ui.painter().layout_job(j);
    let (row, _) = ui.allocate_exact_size(vec2(w, galley.size().y + 2.0 * PAD), Sense::hover());
    ui.painter().rect_filled(row, CornerRadius::same(6), ro.code);
    ui.put(Rect::from_min_size(row.min + vec2(PAD, PAD), galley.size()), egui::Label::new(galley));
    row
}

#[cfg(test)]
#[path = "ai_reply_tests.rs"]
mod tests;
