//! The committed sample file (`tests/fixtures/penpot/sample.penpot`): rebuilt here and compared
//! byte for byte. `SCRIM_WRITE_PENPOT_FIXTURE=1` writes it instead.

use super::*;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/penpot/sample.penpot");

/// The 8 × 8 photo: a red-to-blue ramp.
fn photo() -> Vec<u8> {
    png(8, 8, |x, y| [255 - (x * 32) as u8, (y * 16) as u8, (x * 32) as u8, 255])
}

/// The 16 × 20 thumbnail of the poster: its background with the card's colour across the middle.
fn thumbnail() -> Vec<u8> {
    png(16, 20, |_, y| if (2..12).contains(&y) { [255, 115, 77, 255] } else { [250, 245, 235, 255] })
}

/// A poster board, a "Button" main component and an instance of it, on one page.
pub fn sample() -> Vec<u8> {
    let photo_id = "0039433d-adc8-430d-b2c3-d884dea6e050";
    let thumb_id = "0039433d-adc8-430d-b2c3-d884dea6e051";
    let page = uuid(0x9001);
    let poster = with(shape(100, "frame", "Poster", (0.0, 0.0, 600.0, 800.0)), "fills", json!([fill("#FAF5EB", 1.0)]));
    let card = shape(101, "rect", "Card", (40.0, 40.0, 520.0, 360.0));
    let card = ["r1", "r2", "r3", "r4"].iter().fold(card, |s, k| with(s, k, json!(24)));
    let card = with(card, "fills", json!([gradient((0.5, 0.0), (0.5, 1.0), &[("#FF734D", 0.0, 1.0), ("#8C33D9", 1.0, 1.0)])]));
    let card = with(card, "shadow", json!([{"style": "drop-shadow", "offsetX": 0, "offsetY": 12, "blur": 32, "spread": 0}]));
    let sun = with(shape(102, "circle", "Sun", (400.0, 80.0, 120.0, 120.0)), "fills", json!([fill("#FFD933", 1.0)]));
    let wave = with(shape(103, "path", "Wave", (40.0, 300.0, 520.0, 100.0)), "content", json!("M40 350 C170 300 430 400 560 350 L560 400 L40 400 Z"));
    let wave = with(wave, "fills", json!([fill("#FFFFFF", 0.25)]));
    let headline = with(shape(104, "text", "Headline", (40.0, 440.0, 520.0, 72.0)), "content", json!({"type": "root", "children": []}));
    let badge = shape(105, "group", "Badge", (40.0, 540.0, 160.0, 40.0));
    let pill = ["r1", "r2", "r3", "r4"].iter().fold(shape(106, "rect", "Pill", (40.0, 540.0, 160.0, 40.0)), |s, k| with(s, k, json!(20)));
    let pill = with(with(pill, "fills", json!([fill("#12121F", 1.0)])), "strokes", json!([{"strokeColor": "#FFFFFF", "strokeWidth": 2}]));
    let dot = with(shape(107, "circle", "Dot", (172.0, 552.0, 16.0, 16.0)), "fills", json!([fill("#FFD933", 1.0)]));
    let photo_layer = shape(108, "image", "Photo", (360.0, 520.0, 200.0, 200.0));
    let photo_layer = with(photo_layer, "metadata", json!({"width": 8, "height": 8, "mtype": "image/png", "id": photo_id}));
    let cta = with(shape(109, "frame", "Call to action", (40.0, 680.0, 200.0, 56.0)), "componentId", json!(uuid(0x7001)));
    let cta = with(cta, "componentFile", json!(FILE));
    let cta_bg =
        ["r1", "r2", "r3", "r4"].iter().fold(shape(110, "rect", "Button background", (40.0, 680.0, 200.0, 56.0)), |s, k| with(s, k, json!(28)));
    let cta_bg = with(with(cta_bg, "fills", json!([fill("#12121F", 1.0)])), "shapeRef", json!(uuid(201)));
    let poster = nest(
        poster,
        vec![
            vec![card],
            vec![sun],
            vec![wave],
            vec![headline],
            nest(badge, vec![vec![pill], vec![dot]]),
            vec![photo_layer],
            nest(cta, vec![vec![cta_bg]]),
        ],
    );
    let button = with(shape(200, "frame", "Button", (700.0, 0.0, 200.0, 56.0)), "componentId", json!(uuid(0x7001)));
    let button_bg =
        ["r1", "r2", "r3", "r4"].iter().fold(shape(201, "rect", "Button background", (700.0, 0.0, 200.0, 56.0)), |s, k| with(s, k, json!(28)));
    let button = nest(button, vec![vec![with(button_bg, "fills", json!([fill("#12121F", 1.0)]))]]);
    let thumb = json!({"fileId": FILE, "pageId": page, "frameId": uuid(100), "tag": "frame", "mediaId": thumb_id});
    Export::new()
        .object(photo_id, "png", "image/png", photo())
        .object(thumb_id, "png", "image/png", thumbnail())
        .json(&format!("files/{FILE}/thumbnails/frame/{page}/{}.json", uuid(100)), &thumb)
        .component(1, "Button", 200)
        .page(1, "Poster", json!({"index": 0, "background": "#E8E8E8"}), [poster, button].concat())
        .bytes()
}

#[test]
fn the_committed_sample_matches_the_builder() {
    let bytes = sample();
    if std::env::var("SCRIM_WRITE_PENPOT_FIXTURE").as_deref() == Ok("1") {
        std::fs::write(FIXTURE, &bytes).unwrap();
    }
    let committed = std::fs::read(FIXTURE).expect("tests/fixtures/penpot/sample.penpot (write it with SCRIM_WRITE_PENPOT_FIXTURE=1)");
    assert!(committed == bytes, "sample.penpot differs from the builder; rewrite it with SCRIM_WRITE_PENPOT_FIXTURE=1");
    assert!(is_penpot(&committed));
    assert_eq!(preview(&committed), Some(thumbnail()));
}

#[test]
fn the_sample_imports_with_its_report() {
    let (doc, w) = import(&sample(), "Sample").unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    let names: Vec<&str> = doc.pages.iter().map(|p| doc.node(*p).unwrap().name.as_str()).collect();
    assert_eq!(names, ["Poster"]);
    assert!(matches!(named(&doc, "Poster").kind, NodeKind::Frame));
    assert!(matches!(named(&doc, "Card").fills[0].kind, PaintKind::Gradient(_)));
    assert_eq!(named(&doc, "Card").corners.radii, [24.0; 4]);
    assert!(matches!(named(&doc, "Sun").kind, NodeKind::Ellipse { .. }));
    assert!(matches!(&named(&doc, "Wave").kind, NodeKind::Vector { network } if !network.segments.is_empty()));
    assert!(matches!(named(&doc, "Badge").kind, NodeKind::Group));
    assert!(matches!(named(&doc, "Photo").fills[0].kind, PaintKind::Image(_)));
    assert!(matches!(named(&doc, "Button").kind, NodeKind::Component { .. }));
    assert!(matches!(named(&doc, "Call to action").kind, NodeKind::Frame));
    let mut report: Vec<(Severity, &str)> = w.iter().map(|x| (x.severity, x.path.as_str())).collect();
    report.sort();
    assert_eq!(
        report,
        [
            (Severity::Dropped, "Poster / Poster / Badge / Pill"),
            (Severity::Dropped, "Poster / Poster / Card"),
            (Severity::Dropped, "Poster / Poster / Headline"),
            (Severity::Approximated, "Poster / Poster / Call to action"),
            (Severity::Info, ""),
        ],
        "{w:?}"
    );
}
