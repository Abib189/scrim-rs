//! Port 7: Scrim's layout guides against FigCraft's layout grids. FigCraft already has Figma's
//! layout grids (`node.grids`, `figcraft_layout::grid_lines`), the model Scrim's guides copied,
//! so this test maps each of Scrim's 29 geometry cases (`docs/scrim-transfer/cases/guides.json`)
//! onto a `LayoutGrid` and checks both claims of `docs/scrim-ports/layout-guides.md`:
//! - **the same:** where the two agree (every track position and size), to 1e-6;
//! - **the differences:** each one exactly as listed (edge lines on square grids, touching bands
//!   at gutter 0, zero-size bands when nothing fits, snap lines past the frame).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use figcraft_color::Color;
use figcraft_doc::{GridAlign, GridKind, LayoutGrid};
use serde_json::Value;

fn cases() -> Vec<Value> {
    let v: Value = serde_json::from_str(include_str!("../../../docs/scrim-transfer/cases/guides.json")).unwrap();
    v["geometry"].as_array().unwrap().clone()
}

/// Scrim's guide as FigCraft's layout grid (`layout-guides.md` §6.1).
fn grid(g: &Value) -> LayoutGrid {
    let kind = match g["type"].as_str().unwrap() {
        "column" => GridKind::Columns,
        "row" => GridKind::Rows,
        _ => GridKind::Grid,
    };
    let align = match g["align"].as_str().unwrap_or("stretch") {
        "start" => GridAlign::Min,
        "end" => GridAlign::Max,
        "center" => GridAlign::Center,
        _ => GridAlign::Stretch,
    };
    let hex = g["color"].as_str().unwrap();
    let c = Color::from_hex(hex).unwrap().with_alpha(g["opacity"].as_f64().unwrap() as f32);
    LayoutGrid {
        kind,
        size: g["size"].as_f64().unwrap(),
        count: g["count"].as_u64().unwrap_or(5) as u32,
        gutter: g["gutter"].as_f64().unwrap_or(0.0),
        offset: g["margin"].as_f64().unwrap_or(0.0),
        align,
        color: c,
        visible: g["visible"].as_bool().unwrap(),
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn scrims_guides_map_onto_figcrafts_layout_grids() {
    let (mut tracks, mut grids) = (0, 0);
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let f = &case["frame"];
        let (fx, fy, fw, fh) = (f["x"].as_f64().unwrap(), f["y"].as_f64().unwrap(), f["width"].as_f64().unwrap(), f["height"].as_f64().unwrap());
        for (g, want) in case["guides"].as_array().unwrap().iter().zip(case["perGuide"].as_array().unwrap()) {
            let lg = grid(g);
            let rects = figcraft_layout::grid_lines(&lg, fw, fh);
            match lg.kind {
                GridKind::Columns | GridKind::Rows => {
                    tracks += 1;
                    let cols = lg.kind == GridKind::Columns;
                    let t = &want["track"];
                    let (start, size, step) = (t["start"].as_f64().unwrap(), t["size"].as_f64().unwrap(), t["step"].as_f64().unwrap());
                    // The same track formulas: FigCraft's i-th rectangle is Scrim's i-th track.
                    assert_eq!(rects.len(), g["count"].as_u64().unwrap() as usize, "{name}");
                    for (i, r) in rects.iter().enumerate() {
                        let (a, w) = if cols { (r.x0, r.width()) } else { (r.y0, r.height()) };
                        assert!(close(a, start + i as f64 * step), "{name}: track {i} at {a}, Scrim {}", start + i as f64 * step);
                        assert!(close(w, size), "{name}: track {i} is {w}, Scrim {size}");
                    }
                    let origin = if cols { fx } else { fy };
                    if size <= 0.0 {
                        // Difference: nothing fits. Scrim draws nothing; FigCraft keeps zero-size tracks.
                        assert!(want["bands"].as_array().unwrap().is_empty() && want["xs"].as_array().unwrap().is_empty(), "{name}");
                        assert!(rects.iter().all(|r| if cols { r.width() == 0.0 } else { r.height() == 0.0 }), "{name}");
                    } else if lg.gutter == 0.0 {
                        // Difference: Scrim draws the band edges as lines; FigCraft draws touching
                        // bands whose edges are those lines.
                        let lines: Vec<f64> = want[if cols { "xs" } else { "ys" }].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
                        let mut edges: Vec<f64> =
                            rects.iter().flat_map(|r| if cols { [r.x0, r.x1] } else { [r.y0, r.y1] }).map(|v| v + origin).collect();
                        edges.dedup_by(|a, b| close(*a, *b));
                        assert!(edges.len() == lines.len() && edges.iter().zip(&lines).all(|(a, b)| close(*a, *b)), "{name}: {edges:?} vs {lines:?}");
                    } else {
                        // The same bands, in world coordinates.
                        let bands = want["bands"].as_array().unwrap();
                        assert_eq!(bands.len(), rects.len(), "{name}");
                        for (b, r) in bands.iter().zip(&rects) {
                            let key = if cols { "x" } else { "y" };
                            assert!(close(b[key].as_f64().unwrap(), if cols { r.x0 + fx } else { r.y0 + fy }), "{name}");
                        }
                    }
                }
                GridKind::Grid => {
                    grids += 1;
                    let fig_xs: Vec<f64> = rects.iter().filter(|r| r.x0 == r.x1).map(|r| r.x0 + fx).collect();
                    let scrim_xs: Vec<f64> = want["xs"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
                    // Difference: FigCraft also draws the frame's own left edge, and the right one
                    // when the size divides the width; Scrim draws interior lines only.
                    let interior: Vec<f64> = fig_xs.iter().copied().filter(|x| *x > fx + 1e-6 && *x < fx + fw - 1e-6).collect();
                    assert!(
                        interior.len() == scrim_xs.len() && interior.iter().zip(&scrim_xs).all(|(a, b)| close(*a, *b)),
                        "{name}: {interior:?} vs {scrim_xs:?}"
                    );
                    assert!(close(fig_xs[0], fx), "{name}: FigCraft starts at the frame's edge");
                }
            }
        }
    }
    // Every column, row and grid guide in Scrim's geometry cases is covered.
    assert_eq!((tracks, grids), (25, 8));
}

/// Snapping: both snap only to visible guides and to the same edges, but Scrim drops lines
/// outside the frame and FigCraft keeps them.
#[test]
fn snap_lines_agree_inside_the_frame() {
    use figcraft_doc::{Document, NodeKind};
    for case in cases() {
        let name = case["name"].as_str().unwrap();
        let f = &case["frame"];
        let (fx, fy, fw, fh) = (f["x"].as_f64().unwrap(), f["y"].as_f64().unwrap(), f["width"].as_f64().unwrap(), f["height"].as_f64().unwrap());
        let mut doc = Document::new("Guides");
        let page = doc.pages[0];
        let frame = doc.create(page, NodeKind::Frame, "Frame", fx, fy, fw, fh).unwrap();
        doc.node_mut(frame).unwrap().grids = case["guides"].as_array().unwrap().iter().map(grid).collect();
        let (xs, ys) = figcraft_layout::grid_snap_lines(&doc, frame);
        for t in case["snapTargets"].as_array().unwrap() {
            let (vertical, v) = if t["width"].as_f64() == Some(0.0) { (true, t["x"].as_f64().unwrap()) } else { (false, t["y"].as_f64().unwrap()) };
            let have = if vertical { &xs } else { &ys };
            assert!(have.iter().any(|x| close(*x, v)), "{name}: Scrim snaps to {v}, FigCraft doesn't ({have:?})");
        }
        let outside = xs.iter().any(|x| *x < fx - 1e-6 || *x > fx + fw + 1e-6) || ys.iter().any(|y| *y < fy - 1e-6 || *y > fy + fh + 1e-6);
        if case["guides"].as_array().unwrap().iter().all(|g| g["visible"] == false) {
            assert!(xs.is_empty() && ys.is_empty(), "{name}: a hidden grid doesn't snap in FigCraft either");
        }
        if name.starts_with("fixed columns past the frame") {
            assert!(outside, "{name}: FigCraft keeps the lines past the frame");
        }
    }
}
