//! Auto layout: direction, spacing, padding, alignment, wrapping and how children size.

use figcraft_doc::{CrossAlign, LayoutMode, MainAlign, Sizing};

use super::*;

fn frame_with(id: u32, name: &str, auto_layout: Value, kids: Vec<Value>) -> Value {
    with(container("FRAME", id, name, [0.0, 0.0, 400.0, 300.0], kids), "autoLayout", auto_layout)
}

#[test]
fn a_horizontal_auto_layout() {
    let fill = with(layer("RECT", 2, "Grows", [16.0, 8.0, 100.0, 40.0]), "stretchWidth", json!(true));
    let fixed = with(with(layer("RECT", 3, "Fixed", [124.0, 8.0, 60.0, 40.0]), "minWidth", json!(40)), "maxWidth", json!(80));
    let pinned = with(layer("RECT", 4, "Pinned", [0.0, 0.0, 10.0, 10.0]), "absolutePos", json!(true));
    let label = text_layer(5, "Label", [192.0, 8.0, 50.0, 20.0], "Hi", "Inter-Regular", 14.0, "000");
    let al = json!({"spacing": 8, "padding": [16, 8, 24, 4], "align": 3, "valign": 1, "fixWidth": true});
    let (doc, w) = one(vec![frame_with(1, "Row", al, vec![fill, fixed, pinned, label])]);
    let row = named(&doc, "Row");
    let l = row.layout.as_ref().unwrap();
    assert_eq!((l.mode, l.gap, l.wrap), (LayoutMode::Horizontal, 8.0, false));
    assert_eq!(l.padding, [8.0, 24.0, 4.0, 16.0], "[top, right, bottom, left] from [l, t, r, b]");
    assert_eq!((l.main_align, l.cross_align), (MainAlign::SpaceBetween, CrossAlign::Center));
    assert!(!l.strokes_included, "the README's default");
    assert_eq!((row.child_layout.sizing_h, row.child_layout.sizing_v), (Sizing::Fixed, Sizing::Hug));
    assert_eq!(named(&doc, "Grows").child_layout.sizing_h, Sizing::Fill);
    let f = &named(&doc, "Fixed").child_layout;
    assert_eq!((f.sizing_h, f.min_w, f.max_w, f.min_h), (Sizing::Fixed, Some(40.0), Some(80.0), None));
    assert!(named(&doc, "Pinned").child_layout.absolute);
    let t = &named(&doc, "Label").child_layout;
    assert_eq!((t.sizing_h, t.sizing_v), (Sizing::Hug, Sizing::Hug), "flexible text hugs");
    assert_eq!(pos(named(&doc, "Fixed")), (124.0, 8.0), "children keep the positions the file gives them");
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn a_vertical_wrapping_auto_layout_hugging_inside_another() {
    let a = with(layer("RECT", 3, "A", [0.0, 0.0, 50.0, 50.0]), "stretchHeight", json!(true));
    let inner_al =
        json!({"vertical": true, "spacing": 4, "wrapSpacing": 6, "wrap": true, "align": 2, "valign": 1, "padding": [10], "strokesIncluded": true});
    let inner = with(frame_with(2, "Column", inner_al, vec![a]), "stretchWidth", json!(true));
    let (doc, w) = one(vec![frame_with(1, "Outer", json!({"vertical": true}), vec![inner])]);
    let c = named(&doc, "Column");
    let l = c.layout.as_ref().unwrap();
    assert_eq!((l.mode, l.gap, l.cross_gap, l.wrap), (LayoutMode::Vertical, 4.0, 6.0, true));
    assert_eq!(l.padding, [10.0; 4]);
    // Vertical: `valign` is the main axis, `align` the cross axis.
    assert_eq!((l.main_align, l.cross_align, l.strokes_included), (MainAlign::Center, CrossAlign::Max, true));
    assert_eq!((c.child_layout.sizing_h, c.child_layout.sizing_v), (Sizing::Fill, Sizing::Hug), "hug, or fill when stretched");
    assert_eq!(named(&doc, "A").child_layout.sizing_v, Sizing::Fill);
    let o = named(&doc, "Outer");
    assert_eq!((o.child_layout.sizing_h, o.child_layout.sizing_v), (Sizing::Hug, Sizing::Hug));
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn baseline_alignment_and_the_stacking_option() {
    let al = json!({"textBaseline": true, "reverseZIndex": true, "valign": 3});
    let (doc, w) = one(vec![frame_with(1, "Row", al, vec![])]);
    assert_eq!(named(&doc, "Row").layout.as_ref().unwrap().cross_align, CrossAlign::Baseline);
    assert!(has(&w, Severity::Dropped, "Page 1 / Row", "stacking"), "{w:?}");
}
