//! The HTML export draws what the canvas draws (the export guarantee, Port 2): each fix found by
//! `cargo xtask fidelity` has its case here, checked on the generated code. The browser
//! comparison itself is the xtask.

use serde_json::json;

use super::*;
use crate::Session;
use crate::params::id_of;

fn html_of(s: &Session, id: NodeId) -> String {
    code_for(s.doc().unwrap(), id, CodeLang::Html)
}

fn frame(s: &mut Session, w: u32, h: u32) -> NodeId {
    id_of(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": w, "height": h})).unwrap()["id"]).unwrap()
}

#[test]
fn sizes_include_padding_and_paragraphs_add_no_margin() {
    let mut s = Session::new();
    let f = frame(&mut s, 200, 100);
    s.execute("text.create", &json!({"text": "Hi", "x": 0, "y": 0, "parent": f})).unwrap();
    let html = html_of(&s, f);
    assert!(html.contains("*, *::before, *::after {\n  box-sizing: border-box;\n}\np {\n  margin: 0;\n}\n"), "{html}");
}

#[test]
fn paints_stack_as_background_layers_top_first() {
    let mut s = Session::new();
    let f = frame(&mut s, 100, 100);
    s.execute("node.set", &json!({"id": f, "props": {"fills": ["#FF0000", "#0000FF80"]}})).unwrap();
    let html = html_of(&s, f);
    // The blue paint is on top, so it is the first layer.
    let bg = html.lines().find(|l| l.trim_start().starts_with("background:")).unwrap();
    let (blue, red) = (bg.find("0, 0, 255").or_else(|| bg.find("#0000FF")).unwrap(), bg.find("#FF0000").unwrap());
    assert!(blue < red, "{bg}");
    assert!(bg.contains("linear-gradient(#FF0000, #FF0000)"), "{bg}");
}

#[test]
fn image_paints_are_inlined_as_data_uris_with_their_scale_mode() {
    let mut s = Session::new();
    let f = frame(&mut s, 100, 100);
    // An 8×8 PNG.
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAEklEQVR4nGP4z8CAFWEXHbQEAN9VD/EJ4ZOJAAAAAElFTkSuQmCC";
    s.execute("image.place", &json!({"data": png, "x": 0, "y": 0, "width": 50, "height": 50, "parent": f, "mode": "fit"})).unwrap();
    let html = html_of(&s, f);
    assert!(html.contains("url(data:image/png;base64,iVBORw0KGgo"), "{html}");
    assert!(html.contains("center / contain no-repeat"), "{html}");
    assert!(!html.contains("url(image.png)"), "{html}");
}

#[test]
fn shapes_css_cant_draw_are_inline_svg_turned_or_not() {
    let mut s = Session::new();
    let f = frame(&mut s, 200, 200);
    s.execute("vector.create", &json!({"path": "M0 0 L40 0 L20 30 Z", "x": 10, "y": 10, "parent": f})).unwrap();
    let line = id_of(&s.execute("shape.line", &json!({"x": 20, "y": 100, "width": 120, "parent": f})).unwrap()["id"]).unwrap();
    s.execute("node.set", &json!({"id": line, "props": {"rotation": 30}})).unwrap();
    let arc = id_of(&s.execute("shape.ellipse", &json!({"x": 100, "y": 10, "width": 40, "height": 40, "parent": f})).unwrap()["id"]).unwrap();
    s.execute("node.set", &json!({"id": arc, "props": {"arcSweep": 50}})).unwrap_or_default();
    let html = html_of(&s, f);
    assert!(html.matches("<svg").count() >= 2, "the path and the line: {html}");
    assert!(html.contains("position: absolute; left:"), "placed over the layer's box: {html}");
    // The line's turn is in its SVG, not on its box.
    let line_rule = html.split(".line {").nth(1).unwrap_or_default().split('}').next().unwrap_or_default();
    assert!(!line_rule.contains("rotate("), "{line_rule}");
}

#[test]
fn a_mask_clips_the_layers_above_it_and_is_not_drawn() {
    let mut s = Session::new();
    let f = frame(&mut s, 300, 200);
    s.execute("shape.rectangle", &json!({"x": 0, "y": 0, "width": 20, "height": 20, "parent": f})).unwrap();
    let m = id_of(&s.execute("shape.rectangle", &json!({"x": 50, "y": 40, "width": 100, "height": 80, "parent": f})).unwrap()["id"]).unwrap();
    s.execute("object.mask", &json!({"ids": [m]})).unwrap();
    let html = html_of(&s, f);
    assert!(html.contains("clip-path: inset(40px 150px 80px 50px);"), "{html}");
}

#[test]
fn text_shadows_come_from_the_glyphs() {
    let mut s = Session::new();
    let f = frame(&mut s, 200, 100);
    let t = id_of(&s.execute("text.create", &json!({"text": "Hi", "x": 0, "y": 0, "parent": f})).unwrap()["id"]).unwrap();
    s.execute("effect.add", &json!({"ids": [t], "type": "dropShadow"})).unwrap();
    let html = html_of(&s, f);
    assert!(html.contains("text-shadow:"), "{html}");
    assert!(!html.contains("box-shadow:"), "{html}");
}

#[test]
fn auto_spacing_replaces_the_gap_and_wrapped_lines_keep_theirs() {
    let mut s = Session::new();
    let f = frame(&mut s, 400, 60);
    s.execute("layout.add", &json!({"ids": [f], "direction": "horizontal"})).unwrap();
    s.execute("node.set", &json!({"ids": [f], "sizingH": "fixed", "width": 400, "gap": 12, "mainAlign": "spaceBetween"})).unwrap();
    let html = html_of(&s, f);
    assert!(html.contains("gap: 0px;") && html.contains("justify-content: space-between;"), "{html}");
    s.execute("node.set", &json!({"ids": [f], "mainAlign": "min", "wrap": true})).unwrap();
    let html = html_of(&s, f);
    assert!(html.contains("flex-wrap: wrap;") && html.contains("gap: 10px 12px;"), "{html}");
}
