//! Text layers: the `attributedString` (string and attribute runs), the layer's `textStyle` and
//! `textBehaviour`, as text content with styled runs.

use figcraft_doc::color::{Color, Paint};
use figcraft_doc::{
    LetterSpacing, LineHeight, Node, NodeKind, StyleRun, TextAlignH, TextAlignV, TextAutoResize, TextCase, TextData, TextDecoration, TextStyle,
};
use serde_json::Value;

use super::Ctx;
use super::json::{self, MAX_SIZE, clip, flag, int, list, num, text};
use crate::import_report::Severity;

/// Most characters read from one text layer.
const MAX_CHARS: usize = 1 << 20;
/// Most attribute runs read from one text layer.
const MAX_RUNS: usize = 10_000;

/// Paragraph settings of an attribute run (the model keeps one set per layer).
#[derive(Clone, Copy, PartialEq)]
struct Para {
    align: Option<i64>,
    spacing: Option<f64>,
}

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

/// Family and style names from a PostScript font name (`"Inter-SemiBoldItalic"` → `"Inter"`,
/// `"Semi Bold Italic"`). Trailing `MT` / `PS` vendor marks are left out.
pub(super) fn font_names(postscript: &str) -> (String, String) {
    let (family, style) = postscript.split_once('-').unwrap_or((postscript, ""));
    let vendor = |w: &&str| !matches!(*w, "MT" | "PS" | "PSMT");
    let family: Vec<String> = split_camel(family).split_whitespace().filter(vendor).map(str::to_string).collect();
    let mut words: Vec<String> = Vec::new();
    for w in split_camel(style).split_whitespace().filter(vendor) {
        let mapped: &[&str] = match w.to_ascii_lowercase().as_str() {
            "oblique" | "italic" | "it" => &["Italic"],
            "regular" | "roman" | "book" | "normal" | "plain" => &[],
            "semibold" | "demibold" => &["Semi", "Bold"],
            "extrabold" | "ultrabold" => &["Extra", "Bold"],
            "extralight" | "ultralight" => &["Extra", "Light"],
            "demi" => &["Semi"],
            "ultra" => &["Extra"],
            _ => {
                words.push(w.to_string());
                continue;
            }
        };
        words.extend(mapped.iter().map(|m| m.to_string()));
    }
    let family = if family.is_empty() { postscript.to_string() } else { family.join(" ") };
    let style = if words.is_empty() { "Regular".to_string() } else { words.join(" ") };
    (family, style)
}

/// A 4-character OpenType axis tag from its numeric identifier (`2003265652` → `"wght"`).
fn axis_tag(id: &str) -> Option<String> {
    let n: u32 = id.parse().ok()?;
    let tag: String = n.to_be_bytes().iter().map(|b| char::from(*b)).collect();
    tag.chars().all(|c| c.is_ascii_alphanumeric()).then(|| tag.to_ascii_lowercase())
}

impl Ctx<'_> {
    /// Set `n`'s text (and its colour fill) from the text layer `j`.
    pub(super) fn text(&mut self, n: &mut Node, j: &Value, path: &str) {
        let a = j.get("attributedString").unwrap_or(&Value::Null);
        let raw = text(a, "string").unwrap_or("");
        let characters: String = raw.chars().take(MAX_CHARS).collect();
        if characters.len() < raw.len() {
            self.warn(Severity::Approximated, path, "Very long text was cut to 1,048,576 characters.");
        }
        let text_style = j.get("style").and_then(|s| s.get("textStyle"));
        let encoded = text_style.and_then(|t| t.get("encodedAttributes"));
        let runs_json: Vec<&Value> = list(a, "attributes").iter().take(MAX_RUNS).collect();
        if list(a, "attributes").len() > MAX_RUNS {
            self.warn(Severity::Approximated, path, format!("Text styles past the first {MAX_RUNS} runs use the first style."));
        }
        // The first run sets the layer's style; the text style stands in when there are no runs.
        let first = runs_json.first().and_then(|r| r.get("attributes")).or(encoded);
        if first.is_none() {
            self.warn(Severity::Approximated, path, "The text has no text attributes; it uses the default font.");
        }
        let (base, base_color, para) = self.attrs(first.unwrap_or(&Value::Null), &TextStyle::default(), path);
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
        let mut runs = Vec::new();
        let mut paras = vec![para];
        for r in &runs_json {
            let (Some(loc), Some(len)) = (int(r, "location"), int(r, "length")) else {
                self.warn(Severity::Approximated, path, "A text style run without a range was skipped.");
                continue;
            };
            let (start, end) = (to_char(loc.max(0)), to_char(loc.max(0).saturating_add(len.max(0))));
            let (mut style, color, p) = self.attrs(r.get("attributes").unwrap_or(&Value::Null), &base, path);
            if color != base_color {
                style.fills = Some(vec![solid(color)]);
            }
            paras.push(p);
            if start < end && style != base {
                runs.push(StyleRun { start, end, style });
            }
        }
        if paras.iter().any(|p| p.align != para.align) {
            self.warn(Severity::Approximated, path, "Paragraphs with different alignments all use the first one's.");
        }
        if paras.iter().any(|p| p.spacing != para.spacing) {
            self.warn(Severity::Approximated, path, "Paragraphs with different spacing all use the first one's.");
        }
        let mut td = TextData { characters, style: base, runs, ..TextData::default() };
        td.align_h = match para.align {
            Some(1) => TextAlignH::Right,
            Some(2) => TextAlignH::Center,
            Some(3) => TextAlignH::Justified,
            _ => TextAlignH::Left,
        };
        let v_align = text_style.and_then(|t| int(t, "verticalAlignment")).or_else(|| first.and_then(|f| int(f, "textStyleVerticalAlignmentKey")));
        td.align_v = match v_align {
            Some(1) => TextAlignV::Middle,
            Some(2) => TextAlignV::Bottom,
            _ => TextAlignV::Top,
        };
        td.paragraph_spacing = para.spacing.unwrap_or(0.0);
        td.auto_resize = match int(j, "textBehaviour") {
            Some(1) => TextAutoResize::Height,
            Some(2) => TextAutoResize::None,
            _ => TextAutoResize::WidthAndHeight,
        };
        td.normalize();
        if flag(j, "automaticallyDrawOnUnderlyingPath") == Some(true) {
            self.warn(Severity::Approximated, path, "Text on a path was imported as text in its box.");
        }
        n.kind = NodeKind::Text(td);
        n.fills = vec![solid(base_color)];
    }

    /// The style `v` (string attributes or a text style's encoded attributes) gives over `base`,
    /// with its colour and paragraph settings.
    fn attrs(&mut self, v: &Value, base: &TextStyle, path: &str) -> (TextStyle, Color, Para) {
        let mut t = base.clone();
        if let Some(f) = v.get("MSAttributedStringFontAttribute").and_then(|f| f.get("attributes")) {
            if let Some(name) = text(f, "name").filter(|n| !n.trim().is_empty()) {
                let (family, style) = font_names(clip(name));
                t.font_family = family;
                t.font_style = style;
            }
            if let Some(size) = num(f, "size").filter(|s| *s > 0.0) {
                t.font_size = size.clamp(0.5, 10_000.0);
            }
            t.variations.clear();
            for (id, value) in f.get("variation").and_then(Value::as_object).into_iter().flatten().take(16) {
                match (axis_tag(id), value.as_f64().filter(|x| x.is_finite())) {
                    (Some(tag), Some(x)) => {
                        t.variations.insert(tag, x.clamp(-1e5, 1e5));
                    }
                    _ => self.warn(Severity::Dropped, path, "A variable font axis setting that couldn't be read was not imported."),
                }
            }
        }
        if let Some(k) = num(v, "kerning") {
            t.letter_spacing = if k == 0.0 { LetterSpacing::default() } else { LetterSpacing::Pixels(k.clamp(-1e4, 1e4)) };
        }
        let p = v.get("paragraphStyle");
        let line = |k: &str| p.and_then(|p| num(p, k)).filter(|x| *x > 0.0).map(|x| x.min(MAX_SIZE));
        match (line("minimumLineHeight"), line("maximumLineHeight")) {
            (Some(lo), Some(hi)) if (lo - hi).abs() > 1e-6 => {
                self.warn(Severity::Approximated, path, "A line height range was imported as its maximum.");
                t.line_height = LineHeight::Pixels(hi);
            }
            (_, Some(h)) | (Some(h), None) => t.line_height = LineHeight::Pixels(h),
            (None, None) => {}
        }
        match int(v, "MSAttributedStringTextTransformAttribute") {
            Some(1) => t.case = TextCase::Upper,
            Some(2) => t.case = TextCase::Lower,
            Some(0) => t.case = TextCase::Original,
            _ => {}
        }
        let underline = int(v, "underlineStyle").map(|u| u != 0);
        let strike = int(v, "strikethroughStyle").map(|s| s != 0);
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
        let color = v.get("MSAttributedStringColorAttribute").and_then(json::color).unwrap_or(Color::BLACK);
        let para =
            Para { align: p.and_then(|p| int(p, "alignment")), spacing: p.and_then(|p| num(p, "paragraphSpacing")).map(|s| s.clamp(0.0, MAX_SIZE)) };
        (t, color, para)
    }
}

/// A solid paint of `c`, its alpha as the paint's opacity.
fn solid(c: Color) -> Paint {
    let mut p = Paint::solid(c.with_alpha(1.0));
    p.opacity = c.a;
    p
}
