//! Text layers: string, font, size, colour, alignment, behaviour and styled runs.

use figcraft_doc::color::PaintKind;
use figcraft_doc::{LetterSpacing, LineHeight, TextAlignH, TextAlignV, TextAutoResize, TextCase, TextData, TextDecoration};

use super::*;

fn data<'a>(doc: &'a Document, name: &str) -> &'a TextData {
    match &named(doc, name).kind {
        NodeKind::Text(t) => t,
        other => panic!("{name} is not text: {other:?}"),
    }
}

#[test]
fn text_keeps_its_string_font_size_colour_and_alignment() {
    let t = text_layer(1, "Title", [0.0, 0.0, 300.0, 40.0], "Hello FREE", "Inter-SemiBoldItalic", 32.0, "1A1A2E");
    let t = with(with(with(t, "align", json!(2)), "valign", json!(1)), "behavior", json!(1));
    let t = with(with(with(t, "kerning", json!(1.5)), "lineHeight", json!(40)), "casing", json!(1));
    let t = with(with(t, "underline", json!(true)), "paragraphSpacing", json!(8));
    let (doc, w) = one(vec![t]);
    let d = data(&doc, "Title");
    assert_eq!(d.characters, "Hello FREE");
    assert_eq!((d.style.font_family.as_str(), d.style.font_style.as_str()), ("Inter", "Semi Bold Italic"));
    assert_eq!(d.style.font_size, 32.0);
    assert_eq!((d.align_h, d.align_v, d.auto_resize), (TextAlignH::Center, TextAlignV::Middle, TextAutoResize::Height));
    assert_eq!((d.style.letter_spacing, d.style.line_height), (LetterSpacing::Pixels(1.5), LineHeight::Pixels(40.0)));
    assert_eq!((d.style.case, d.style.decoration, d.paragraph_spacing), (TextCase::Upper, TextDecoration::Underline, 8.0));
    let PaintKind::Solid { color } = &named(&doc, "Title").fills[0].kind else { panic!() };
    assert_eq!(color.to_rgba8(), [0x1A, 0x1A, 0x2E, 255]);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn the_readme_defaults_apply_to_a_bare_text_layer() {
    let t = with(layer("TEXT", 1, "Bare", [0.0, 0.0, 10.0, 10.0]), "text", json!("x"));
    let (doc, _) = one(vec![t]);
    let d = data(&doc, "Bare");
    assert_eq!((d.style.font_family.as_str(), d.style.font_style.as_str(), d.style.font_size), ("Inter", "Regular", 12.0));
    assert_eq!((d.align_h, d.auto_resize), (TextAlignH::Left, TextAutoResize::WidthAndHeight));
    assert!(named(&doc, "Bare").fills.is_empty(), "no fill: the README's default colour is transparent");
}

#[test]
fn inline_styles_become_runs() {
    let t = text_layer(1, "Mixed", [0.0, 0.0, 300.0, 40.0], "Bold and red", "Inter-Regular", 16.0, "000");
    let inlines = json!([
        {"range": [0, 3], "font": "Inter-Bold"},
        {"range": [9, 11], "fill": "F00", "fontSize": 20}
    ]);
    let (doc, w) = one(vec![with(t, "inlines", inlines)]);
    let d = data(&doc, "Mixed");
    assert_eq!(d.runs.len(), 2, "{:?}", d.runs);
    assert_eq!((d.runs[0].start, d.runs[0].end, d.runs[0].style.font_style.as_str()), (0, 4, "Bold"));
    assert_eq!(d.runs[0].style.font_size, 16.0, "an inline style keeps what it doesn't set");
    let red = &d.runs[1];
    assert_eq!((red.start, red.end, red.style.font_size), (9, 12, 20.0));
    let fills = red.style.fills.as_ref().unwrap();
    let PaintKind::Solid { color } = &fills[0].kind else { panic!() };
    assert_eq!(color.to_rgba8(), [255, 0, 0, 255]);
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn ranges_count_utf16_units() {
    let t = text_layer(1, "Emoji", [0.0, 0.0, 300.0, 40.0], "😀 hi", "Inter-Regular", 16.0, "000");
    // "😀" is two UTF-16 units: "hi" is units 3..=4, characters 2..4.
    let (doc, w) = one(vec![with(t, "inlines", json!([{"range": [3, 4], "font": "Inter-Bold"}]))]);
    let d = data(&doc, "Emoji");
    assert_eq!((d.runs[0].start, d.runs[0].end), (2, 4));
    assert!(has(&w, Severity::Approximated, "Page 1 / Emoji", "UTF-16"), "{w:?}");
}
