//! The committed sample file (`tests/fixtures/free/sample.free`): rebuilt here and compared byte
//! for byte. `SCRIM_WRITE_FREE_FIXTURE=1` writes it instead.

use figcraft_doc::color::PaintKind;
use figcraft_doc::{LayoutMode, Sizing};

use super::super::preview;
use super::*;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/free/sample.free");

/// The 8 × 8 photo: a red-to-blue ramp.
fn photo() -> Vec<u8> {
    png(8, 8, |x, y| [255 - (x * 32) as u8, (y * 16) as u8, (x * 32) as u8, 255])
}

/// The "Button" component: a rounded background and a label, in a horizontal auto layout.
fn button() -> Value {
    let label = text_layer(202, "Button label", [20.0, 16.0, 120.0, 24.0], "Get started", "Inter-SemiBold", 18.0, "FFF");
    let c = container("COMPONENT", 200, "Button", [0.0, 0.0, 160.0, 56.0], vec![label]);
    let c = with(with(c, "componentId", json!(gid(700))), "fill", json!("12121F"));
    let c = with(c, "cornerRadius", json!([28, 28, 28, 28]));
    with(c, "autoLayout", json!({"padding": [20, 16], "align": 1, "valign": 1}))
}

/// A poster page (an auto layout frame) and a components page.
pub fn sample() -> Vec<u8> {
    let gradient =
        json!({"type": 1, "gradient": {"from": [0.5, 0], "to": [0.5, 1], "stops": [{"pos": 0, "color": "FF734D"}, {"pos": 1, "color": "8C33D9"}]}});
    let hero = with(layer("RECT", 101, "Hero", [40.0, 40.0, 520.0, 240.0]), "fills", json!([gradient]));
    let hero = with(with(hero, "cornerRadius", json!([24, 24, 24, 24])), "stretchWidth", json!(true));
    let hero = with(hero, "shadows", json!([{"color": "59000000", "offset": [0, 12], "radius": 32}]));
    let hero = with(hero, "binds", json!([{"field": 2, "numberId": gid(900)}]));
    let sun = with(layer("OVAL", 103, "Sun", [0.0, 0.0, 64.0, 64.0]), "fill", json!("FFD933"));
    let title = text_layer(104, "Title", [80.0, 8.0, 200.0, 48.0], "Scrim reads FREE", "Inter-Black", 40.0, "12121F");
    let title = with(title, "inlines", json!([{"range": [12, 15], "fill": "8C33D9"}]));
    let row = container("FRAME", 102, "Heading", [40.0, 304.0, 296.0, 64.0], vec![sun, title]);
    let row = with(with(row, "autoLayout", json!({"spacing": 16, "valign": 1})), "clipContent", json!(false));
    let photo_layer =
        with(layer("RECT", 105, "Photo", [40.0, 392.0, 200.0, 200.0]), "fills", json!([{"type": 4, "pattern": {"image": "photo.png"}}]));
    let cta = with(layer("INSTANCE", 106, "Call to action", [40.0, 616.0, 160.0, 56.0]), "componentId", json!(gid(700)));
    let cta = with(cta, "overrides", json!([{"target": [gid(202)], "text": "Open it"}]));
    let cta = with(cta, "flows", json!([{"trigger": {"_t": "CLICK"}, "actions": [{"_t": "URL", "url": "https://example.com"}]}]));
    let note = layer("CONNECTOR", 107, "Note arrow", [0.0, 0.0, 10.0, 10.0]);
    let poster = container("FRAME", 100, "Poster art", [0.0, 0.0, 600.0, 712.0], vec![hero, row, photo_layer, cta, note]);
    let poster = with(poster, "fill", json!("FAF5EB"));
    let poster = with(poster, "autoLayout", json!({"vertical": true, "spacing": 24, "padding": [40], "fixWidth": true}));
    let preview_webp = webp(30, 36, |_, y| [255, (160 + y * 2) as u8, 90, 255]);
    free(
        &[page(1, "Poster", vec![poster]), page(2, "Components", vec![button()])],
        json!({}),
        &[("images/photo.png", &photo()), ("preview.webp", &preview_webp)],
    )
}

#[test]
fn the_committed_sample_matches_the_builder() {
    let bytes = sample();
    if std::env::var("SCRIM_WRITE_FREE_FIXTURE").as_deref() == Ok("1") {
        std::fs::write(FIXTURE, &bytes).unwrap();
    }
    let committed = std::fs::read(FIXTURE).expect("tests/fixtures/free/sample.free (write it with SCRIM_WRITE_FREE_FIXTURE=1)");
    assert!(committed == bytes, "sample.free differs from the builder; rewrite it with SCRIM_WRITE_FREE_FIXTURE=1");
    assert!(is_free(&committed));
    assert!(preview(&committed).is_some_and(|p| image::load_from_memory(&p).is_ok()));
}

#[test]
fn the_sample_imports_with_its_report() {
    let (doc, w) = import(&sample(), "Sample").unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    let names: Vec<&str> = doc.pages.iter().map(|p| doc.node(*p).unwrap().name.as_str()).collect();
    assert_eq!(names, ["Poster", "Components"]);
    let poster = named(&doc, "Poster art");
    assert_eq!(poster.layout.as_ref().map(|l| l.mode), Some(LayoutMode::Vertical));
    assert_eq!((poster.child_layout.sizing_h, poster.child_layout.sizing_v), (Sizing::Fixed, Sizing::Hug));
    assert_eq!(named(&doc, "Hero").child_layout.sizing_h, Sizing::Fill);
    assert!(matches!(named(&doc, "Hero").fills[0].kind, PaintKind::Gradient(_)));
    assert_eq!(named(&doc, "Hero").corners.radii, [24.0; 4]);
    let heading = named(&doc, "Heading");
    assert_eq!(heading.parent, Some(poster.id));
    assert_eq!((heading.child_layout.sizing_h, heading.child_layout.sizing_v), (Sizing::Hug, Sizing::Hug));
    assert!(matches!(named(&doc, "Sun").kind, NodeKind::Ellipse { .. }));
    assert!(matches!(&named(&doc, "Title").kind, NodeKind::Text(t) if t.runs.len() == 1));
    assert!(matches!(named(&doc, "Photo").fills[0].kind, PaintKind::Image(_)));
    let cta = named(&doc, "Call to action");
    assert!(matches!(cta.kind, NodeKind::Instance { component, .. } if component == named(&doc, "Button").id));
    let label = doc.subtree(cta.id).into_iter().filter_map(|i| doc.node(i)).find(|n| n.name == "Button label").unwrap();
    assert!(matches!(&label.kind, NodeKind::Text(t) if t.characters == "Open it"));
    let mut report: Vec<(Severity, &str)> = w.iter().map(|x| (x.severity, x.path.as_str())).collect();
    report.sort();
    assert_eq!(
        report,
        [
            (Severity::Dropped, "Poster / Poster art / Call to action"),
            (Severity::Dropped, "Poster / Poster art / Hero"),
            (Severity::Dropped, "Poster / Poster art / Note arrow"),
        ],
        "{w:?}"
    );
}
