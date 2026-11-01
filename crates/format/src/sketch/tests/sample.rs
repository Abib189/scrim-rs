//! The committed sample file (`tests/fixtures/sketch/sample.sketch`): rebuilt here and compared
//! byte for byte. `SCRIM_WRITE_SKETCH_FIXTURE=1` writes it instead.

use super::*;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sketch/sample.sketch");

/// The 8 × 8 photo: a red-to-blue ramp.
fn photo() -> Vec<u8> {
    png(8, 8, |x, y| [255 - (x * 32) as u8, (y * 16) as u8, (x * 32) as u8, 255])
}

/// A poster page and a symbols page.
pub fn sample() -> Vec<u8> {
    let white = || color(1.0, 1.0, 1.0, 1.0);
    let ink = || color(0.07, 0.07, 0.12, 1.0);
    let card = rectangle(101, "Card", frame(40.0, 40.0, 520.0, 360.0), [24.0; 4]);
    let card =
        styled(card, "fills", json!([gradient(0, "{0.5, 0}", "{0.5, 1}", &[(0.0, color(1.0, 0.45, 0.3, 1.0)), (1.0, color(0.55, 0.2, 0.85, 1.0))])]));
    let card = styled(card, "borders", json!([border(white(), 2.0, 1)]));
    let card = styled(card, "shadows", json!([shadow("shadow", color(0.0, 0.0, 0.0, 0.35), 0.0, 12.0, 32.0, 0.0)]));
    let sun = styled(oval(102, "Sun", frame(400.0, 80.0, 120.0, 120.0)), "fills", json!([solid(color(1.0, 0.85, 0.2, 1.0))]));
    let burst = styled(star(103, "Burst", frame(80.0, 90.0, 90.0, 90.0), 8, 0.6), "fills", json!([solid(white())]));
    let wave_points =
        vec![point(0.0, 0.5, 0.0), curve((0.5, 0.5), (0.25, 0.0), (0.75, 1.0)), point(1.0, 0.5, 0.0), point(1.0, 1.0, 0.0), point(0.0, 1.0, 0.0)];
    let wave = shape("shapePath", 104, "Wave", frame(40.0, 300.0, 520.0, 100.0), wave_points, true);
    let wave = styled(wave, "fills", json!([solid(color(1.0, 1.0, 1.0, 0.25))]));
    let headline = text(
        105,
        "Headline",
        frame(40.0, 440.0, 520.0, 72.0),
        "Scrim imports Sketch",
        vec![(0, 6, attrs("Inter-Black", 56.0, ink(), 0)), (6, 14, attrs("Inter-Regular", 56.0, color(0.55, 0.2, 0.85, 1.0), 0))],
    );
    let badge = container(
        "group",
        106,
        "Badge",
        frame(40.0, 540.0, 160.0, 40.0),
        vec![
            styled(rectangle(107, "Pill", frame(0.0, 0.0, 160.0, 40.0), [20.0; 4]), "fills", json!([solid(ink())])),
            text(108, "Badge label", frame(16.0, 10.0, 128.0, 20.0), "NEW FORMAT", vec![(0, 10, attrs("Inter-Bold", 14.0, white(), 2))]),
        ],
    );
    let photo_layer = with(layer("bitmap", 109, "Photo", frame(360.0, 520.0, 200.0, 200.0)), "image", file_ref("images/photo.png"));
    let cta = instance(110, 7, "Call to action", frame(40.0, 680.0, 200.0, 56.0), vec![(format!("{}_stringValue", uuid(202)), json!("Open it"))]);
    let cta = with(cta, "flow", json!({"_class": "MSImmutableFlowConnection", "destinationArtboardID": uuid(100), "animationType": 0}));
    let hotspot = layer("MSImmutableHotspotLayer", 111, "Tap area", frame(40.0, 680.0, 200.0, 56.0));
    let future = layer("hologram", 112, "Hologram", frame(0.0, 0.0, 10.0, 10.0));
    let poster =
        artboard(100, "Poster", frame(0.0, 0.0, 600.0, 800.0), vec![card, sun, burst, wave, headline, badge, photo_layer, cta, hotspot, future]);
    let poster = with(poster, "backgroundColor", color(0.98, 0.96, 0.92, 1.0));
    let button = master(
        200,
        7,
        "Button",
        frame(0.0, 0.0, 200.0, 56.0),
        vec![
            styled(rectangle(201, "Button background", frame(0.0, 0.0, 200.0, 56.0), [28.0; 4]), "fills", json!([solid(ink())])),
            text(202, "Button label", frame(20.0, 16.0, 160.0, 24.0), "Get started", vec![(0, 11, attrs("Inter-SemiBold", 18.0, white(), 2))]),
        ],
    );
    // Sketch keeps a picture of the document in `previews/preview.png` (Home shows it, Fix AH4).
    let preview = png(30, 40, |_, y| [255, (160 + y * 2) as u8, 90, 255]);
    sketch(
        &[page(1, "Poster", vec![poster]), page(2, "Symbols", vec![button])],
        json!({}),
        &[("images/photo.png", &photo()), ("previews/preview.png", &preview)],
    )
}

#[test]
fn the_committed_sample_matches_the_builder() {
    let bytes = sample();
    if std::env::var("SCRIM_WRITE_SKETCH_FIXTURE").as_deref() == Ok("1") {
        std::fs::write(FIXTURE, &bytes).unwrap();
    }
    let committed = std::fs::read(FIXTURE).expect("tests/fixtures/sketch/sample.sketch (write it with SCRIM_WRITE_SKETCH_FIXTURE=1)");
    assert!(committed == bytes, "sample.sketch differs from the builder; rewrite it with SCRIM_WRITE_SKETCH_FIXTURE=1");
    assert!(is_sketch(&committed));
}

#[test]
fn the_sample_imports_with_its_report() {
    let (doc, w) = import(&sample(), "Sample").unwrap();
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
    let names: Vec<&str> = doc.pages.iter().map(|p| doc.node(*p).unwrap().name.as_str()).collect();
    assert_eq!(names, ["Poster", "Symbols"]);
    assert!(matches!(named(&doc, "Card").kind, NodeKind::Rectangle));
    assert!(matches!(named(&doc, "Sun").kind, NodeKind::Ellipse { .. }));
    assert!(matches!(named(&doc, "Burst").kind, NodeKind::Star { count: 8, .. }));
    assert!(matches!(named(&doc, "Wave").kind, NodeKind::Vector { .. }));
    assert!(matches!(named(&doc, "Badge").kind, NodeKind::Group));
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
            (Severity::Dropped, "Poster / Poster / Call to action"),
            (Severity::Dropped, "Poster / Poster / Hologram"),
            (Severity::Dropped, "Poster / Poster / Tap area"),
        ],
        "{w:?}"
    );
}
