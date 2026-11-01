//! Text layers: the `text` string, the layer's own text fields and its `inlines` (styled runs).
//!
//! The README leaves two things open, read here as follows: an inline style sets what it lists
//! over the layer's style (fields it leaves out are the layer's), and its inclusive `range`
//! counts UTF-16 code units (the strings of the schema's C# writer); text that is all in the
//! Basic Multilingual Plane reads the same either way.

use figcraft_doc::color::Paint;
use figcraft_doc::{
    LetterSpacing, LineHeight, ListStyle, Node, NodeKind, StyleRun, TextAlignH, TextAlignV, TextAutoResize, TextCase, TextData, TextDecoration,
    TextStyle,
};
use serde_json::Value;

use super::Ctx;
use super::json::{MAX_SIZE, clip, flag, int, list, num, text};
use crate::import_report::Severity;

/// Most characters read from one text layer.
const MAX_CHARS: usize = 1 << 20;
/// Most inline styles read from one text layer.
const MAX_RUNS: usize = 10_000;

/// `"SemiBoldItalic"` → `"Semi Bold Italic"`, `"SFProText"` → `"SF Pro Text"`.
fn split_camel(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 4);
    for (i, c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).and_then(|p| chars.get(p));
        let next = chars.get(i + 1);
        let boundary = c.is_uppercase()
            && prev.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit() || (p.is_uppercase() && next.is_some_and(|n| n.is_lowercase())));
        if boundary {
            out.push(' ');
        }
        out.push(*c);
    }
    out
}

/// Family and style from a `font` name in the README's form (`"Inter-Regular"`, a PostScript
/// name): `"Inter-SemiBoldItalic"` → `"Inter"`, `"Semi Bold Italic"`.
pub(super) fn font_names(name: &str) -> (String, String) {
    let (family, style) = name.split_once('-').unwrap_or((name, ""));
    let family = split_camel(family).split_whitespace().collect::<Vec<_>>().join(" ");
    let mut words: Vec<String> = Vec::new();
    for w in split_camel(style).split_whitespace() {
        let mapped: &[&str] = match w.to_ascii_lowercase().as_str() {
            "oblique" | "italic" | "it" => &["Italic"],
            "regular" | "roman" | "book" | "normal" | "plain" => &[],
            "semibold" | "demibold" => &["Semi", "Bold"],
            "extrabold" | "ultrabold" => &["Extra", "Bold"],
            "extralight" | "ultralight" => &["Extra", "Light"],
            _ => {
                words.push(w.to_string());
                continue;
            }
        };
        words.extend(mapped.iter().map(|m| m.to_string()));
    }
    let family = if family.is_empty() { name.to_string() } else { family };
    let style = if words.is_empty() { "Regular".to_string() } else { words.join(" ") };
    (family, style)
}

impl Ctx<'_> {
    /// Set `n`'s text from the text layer `j` (its fills come from the layer's `fill` / `fills`).
    pub(super) fn text(&mut self, n: &mut Node, j: &Value, path: &str) {
        let raw = text(j, "text").unwrap_or("");
        let characters: String = raw.chars().take(MAX_CHARS).collect();
        if characters.len() < raw.len() {
            self.warn(Severity::Approximated, path, "Very long text was cut to 1,048,576 characters.");
        }
        let base = self.text_style(j, &TextStyle::default(), false, path);
        // UTF-16 offsets of each character's start (and the end).
        let mut starts = Vec::with_capacity(characters.len() + 1);
        let mut unit = 0i64;
        for c in characters.chars() {
            starts.push(unit);
            unit += c.len_utf16() as i64;
        }
        starts.push(unit);
        let count = starts.len().saturating_sub(1);
        let to_char = |u: i64| starts.partition_point(|s| *s < u).min(count);
        let inlines = list(j, "inlines");
        if inlines.len() > MAX_RUNS {
            self.warn(Severity::Approximated, path, format!("Text styles past the first {MAX_RUNS} runs use the layer's style."));
        }
        if !inlines.is_empty() && characters.chars().any(|c| c.len_utf16() > 1) {
            self.warn(Severity::Approximated, path, "Styled ranges were counted in UTF-16 code units: the README doesn't give their unit.");
        }
        let mut runs: Vec<StyleRun> = Vec::new();
        for r in inlines.iter().take(MAX_RUNS) {
            let range = r.get("range").and_then(|v| super::json::floats(v, 2));
            let Some([a, b]) = range.as_deref().and_then(|v| <[f64; 2]>::try_from(v).ok()) else {
                self.warn(Severity::Approximated, path, "A text style run without a range was skipped.");
                continue;
            };
            let (a, b) = (a.clamp(0.0, 1e9) as i64, b.clamp(0.0, 1e9) as i64);
            let (start, end) = (to_char(a), to_char(b.saturating_add(1)));
            let style = self.text_style(r, &base, true, path);
            for (k, what) in [("fillsId", "colour"), ("textStyleId", "text")] {
                if r.get(k).is_some() {
                    self.warn(Severity::Info, path, format!("A styled range's link to a {what} style was not kept; it keeps its own values."));
                }
            }
            if !list(r, "binds").is_empty() {
                self.warn(Severity::Dropped, path, "Variable bindings on a styled range were not imported.");
            }
            if start < end && style != base && runs.last().is_none_or(|l| l.end <= start) {
                runs.push(StyleRun { start, end, style });
            } else if start < end && style != base {
                self.warn(Severity::Approximated, path, "Overlapping styled ranges were imported with the first one's style.");
            }
        }
        let mut td = TextData { characters, style: base, runs, ..TextData::default() };
        td.align_h = match int(j, "align").unwrap_or(0) {
            1 | 6 => TextAlignH::Right,
            2 => TextAlignH::Center,
            3 => TextAlignH::Justified,
            0 | 4 | 5 => TextAlignH::Left,
            _ => {
                self.warn(Severity::Approximated, path, "Alignment detected from the text was imported as left-aligned.");
                TextAlignH::Left
            }
        };
        td.align_v = match int(j, "valign").unwrap_or(0) {
            1 => TextAlignV::Middle,
            2 => TextAlignV::Bottom,
            _ => TextAlignV::Top,
        };
        td.paragraph_spacing = num(j, "paragraphSpacing").unwrap_or(0.0).clamp(0.0, MAX_SIZE);
        td.list = match int(j, "list").unwrap_or(0) {
            1 => ListStyle::Bulleted,
            2 => ListStyle::Numbered,
            _ => ListStyle::None,
        };
        let truncate = flag(j, "truncate") == Some(true);
        td.auto_resize = match (int(j, "behavior").unwrap_or(0), truncate) {
            (2, true) => TextAutoResize::Truncate,
            (1, _) => TextAutoResize::Height,
            (2, false) => TextAutoResize::None,
            _ => TextAutoResize::WidthAndHeight,
        };
        if let Some(m) = int(j, "maxLines").filter(|m| *m > 0) {
            td.max_lines = m.clamp(0, 255) as u32;
        }
        td.normalize();
        let notes = [
            (flag(j, "rtl") == Some(true), Severity::Dropped, "Right-to-left text direction was not imported."),
            (int(j, "baselinePos").is_some_and(|b| b != 0), Severity::Dropped, "Superscript or subscript text was not imported."),
            (flag(j, "drawOnPath") == Some(true), Severity::Approximated, "Text on a path was imported as text in its box."),
            (flag(j, "verticalTrim") == Some(true), Severity::Dropped, "Vertical trim of the text box was not imported."),
            (flag(j, "clipContent") == Some(false), Severity::Dropped, "Showing text past its box (clip content off) was not imported."),
            (text(j, "fontOpticalSizing") == Some("auto"), Severity::Dropped, "Automatic optical sizing was not imported."),
        ];
        for (on, severity, message) in notes {
            if on {
                self.warn(severity, path, message);
            }
        }
        if let Some(id) = text(j, "textStyleId") {
            let name = self.style_names.get(clip(id)).cloned();
            match name {
                Some(name) => {
                    self.warn(Severity::Info, path, format!("The link to the text style “{name}” was not kept; the text keeps its own values."))
                }
                None => self.warn(Severity::Dropped, path, "The text's text style isn't in the file; the text keeps its own values."),
            }
        }
        n.kind = NodeKind::Text(td);
    }

    /// The text fields of `v` (a text layer, or an inline style when `inline`) over `base`.
    fn text_style(&mut self, v: &Value, base: &TextStyle, inline: bool, path: &str) -> TextStyle {
        let mut t = base.clone();
        if let Some(font) = text(v, "font").filter(|f| !f.trim().is_empty()) {
            let (family, style) = font_names(clip(font));
            t.font_family = family;
            t.font_style = style;
        }
        if let Some(size) = num(v, "fontSize").filter(|s| *s > 0.0) {
            t.font_size = size.clamp(0.5, 10_000.0);
        }
        if let Some(k) = num(v, "kerning") {
            t.letter_spacing = if k == 0.0 { LetterSpacing::default() } else { LetterSpacing::Pixels(k.clamp(-1e4, 1e4)) };
        }
        if let Some(h) = num(v, "lineHeight").filter(|h| *h > 0.0) {
            t.line_height = LineHeight::Pixels(h.min(MAX_SIZE));
        }
        match int(v, "casing") {
            Some(1) => t.case = TextCase::Upper,
            Some(2) => t.case = TextCase::Lower,
            Some(3) => t.case = TextCase::Title,
            Some(0) => t.case = TextCase::Original,
            _ => {}
        }
        let (underline, strike) = (flag(v, "underline"), flag(v, "strikethrough"));
        match (underline, strike) {
            (Some(true), Some(true)) => {
                self.warn(Severity::Approximated, path, "Text both underlined and struck through was imported underlined only.");
                t.decoration = TextDecoration::Underline;
            }
            (Some(true), _) => t.decoration = TextDecoration::Underline,
            (_, Some(true)) => t.decoration = TextDecoration::Strikethrough,
            (Some(false), _) | (_, Some(false)) => t.decoration = TextDecoration::None,
            (None, None) => {}
        }
        if let Some(vars) = v.get("variation") {
            match vars.as_object() {
                Some(map) => {
                    for (tag, value) in map.iter().take(16) {
                        match value.as_f64().filter(|x| x.is_finite()) {
                            Some(x) if tag.len() == 4 => {
                                t.variations.insert(tag.clone(), x.clamp(-1e5, 1e5));
                            }
                            _ => self.warn(Severity::Dropped, path, "A variable font axis setting that couldn't be read was not imported."),
                        }
                    }
                }
                None => self.warn(Severity::Dropped, path, "The variable font settings couldn't be read and were not imported."),
            }
        }
        // An inline style's fill colours its range; the layer's own fills are the node's.
        if inline && (v.get("fills").is_some() || v.get("fill").is_some()) {
            let fills: Vec<Paint> = self.paints(v, "fills", "fill", path);
            t.fills = Some(fills);
        }
        for (k, what) in [("paragraphSpacing", "paragraph spacing"), ("align", "alignment"), ("valign", "vertical alignment"), ("list", "list style")]
        {
            if inline && num(v, k).is_some_and(|x| x != 0.0) {
                self.warn(Severity::Approximated, path, format!("A styled range's {what} was not imported; the layer's applies to all of it."));
            }
        }
        t
    }
}
