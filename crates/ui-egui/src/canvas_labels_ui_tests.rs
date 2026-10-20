//! E2 in the window (egui headless), by geometry: on the Ridgeline app at 10%, 22%, 50% and 100%
//! (and a sweep of other zooms), with a very long title, every handoff state, a right-to-left
//! title and a narrow frame, no two label areas the canvas publishes overlap. Each sits in its
//! own frame's width just above it, and pressing a label still selects its frame. The label inks
//! read at 4.5:1 as text on both themes' canvases and both default page colours.

use egui::{Pos2, Rect};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::json;

use crate::dev_icon::Ink;
use crate::dev_icon::tests::pages;
use crate::layers_drag_tests::Ui;
use crate::theme::accent::contrast;

/// Ridgeline: Components (a section, x −1300…−447), Home — Light (0…393), Trail — Light
/// (473…866), Home — Dark (946…1339), all from y 0 or 20.
fn ridgeline() -> Ui {
    let mut s = Session::new();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ridgeline-app.figcraft");
    s.execute("file.open", &json!({"path": path})).unwrap();
    s.execute("select.none", &json!({})).unwrap();
    Ui::new(s)
}

fn look(u: &mut Ui, cx: f64, cy: f64, zoom: f64) {
    let ds = u.app.session.ds_mut().unwrap();
    (ds.view.cx, ds.view.cy, ds.view.zoom) = (cx, cy, zoom);
    u.frame(vec![]);
    u.frame(vec![]);
}

/// The label areas the canvas published, on screen.
fn areas(u: &Ui) -> Vec<(NodeId, Rect)> {
    u.app.session.tool.labels.iter().map(|(id, q)| (*id, Rect::from_points(&q.map(|p| u.app.to_screen(p.x, p.y))))).collect()
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x - 0.01 && b.min.x < a.max.x - 0.01 && a.min.y < b.max.y - 0.01 && b.min.y < a.max.y - 0.01
}

/// No two labels overlap, and each sits within its frame's width, above it. The labels checked.
fn check(u: &Ui, at: &str) -> usize {
    let a = areas(u);
    let doc = u.app.session.doc().unwrap();
    for (i, (id, r)) in a.iter().enumerate() {
        let b = doc.world_bounds(*id).unwrap();
        let (tl, br) = (u.app.to_screen(b.x0, b.y0), u.app.to_screen(b.x1, b.y1));
        assert!(r.min.x >= tl.x - 0.5 && r.max.x <= br.x + 0.5, "{at}: {id:?}'s label {r:?} leaves its frame {tl:?}–{br:?}");
        assert!(r.max.y <= tl.y + 0.5 && r.min.y >= tl.y - 18.5, "{at}: {id:?}'s label {r:?} isn't just above {tl:?}");
        for (other, s) in a.iter().skip(i + 1) {
            assert!(!overlap(*r, *s), "{at}: {id:?} {r:?} overlaps {other:?} {s:?}");
        }
    }
    a.len()
}

#[test]
fn ridgeline_labels_never_overlap_at_10_22_50_and_100_percent() {
    let mut u = ridgeline();
    for (zoom, centres) in [
        (0.10, &[(0.0, 450.0)][..]),
        (0.22, &[(0.0, 450.0), (-900.0, 300.0)][..]),
        (0.50, &[(-900.0, 300.0), (650.0, 300.0)][..]),
        (1.00, &[(-1100.0, 150.0), (-700.0, 150.0), (450.0, 150.0), (1150.0, 150.0)][..]),
    ] {
        for (cx, cy) in centres {
            look(&mut u, *cx, *cy, zoom);
            let n = check(&u, &format!("{zoom} at ({cx}, {cy})"));
            assert!(n >= 2, "{zoom} at ({cx}, {cy}): {n} labels");
        }
    }
    // The owner's capture: at 22% Home — Light's label ends inside its frame, Trail's is whole.
    look(&mut u, 0.0, 450.0, 0.22);
    let a = areas(&u);
    let home = a.iter().find(|(id, _)| id.0 == 68).unwrap().1;
    let trail = a.iter().find(|(id, _)| id.0 == 178).unwrap().1;
    assert!(home.max.x + 8.0 <= trail.min.x, "{home:?} {trail:?}");
    // Components (the section) and Landscape (at its corner) both keep a label, apart.
    let comps = a.iter().find(|(id, _)| id.0 == 349).map(|x| x.1);
    let land = a.iter().find(|(id, _)| id.0 == 14).map(|x| x.1);
    assert!(comps.is_some(), "the section's title shows");
    if let (Some(c), Some(l)) = (comps, land) {
        assert!(!overlap(c, l));
    }
}

#[test]
fn a_sweep_of_zooms_never_overlaps() {
    let mut u = ridgeline();
    for k in 0..40 {
        let zoom = 0.04 * 1.12f64.powi(k);
        for cx in [-1000.0, -400.0, 300.0, 1000.0] {
            look(&mut u, cx, 200.0, zoom);
            check(&u, &format!("{zoom:.3} at {cx}"));
        }
    }
}

#[test]
fn long_titles_every_state_rtl_and_narrow_frames_never_overlap() {
    let mut u = ridgeline();
    let s = &mut u.app.session;
    let long = "Home — Dark, the evening variant with every trail card, filter chip and saved route shown";
    s.execute("node.set", &json!({"id": 239, "props": {"name": long, "devStatus": "completed"}})).unwrap();
    s.execute("node.set", &json!({"id": 178, "props": {"name": "مسار — فاتح، النسخة الطويلة", "devStatus": "readyForDev"}})).unwrap();
    s.execute("node.set", &json!({"id": 179, "props": {"opacity": 0.5}})).unwrap();
    s.execute("node.set", &json!({"id": 349, "props": {"devStatus": "readyForDev"}})).unwrap();
    s.execute("frame.create", &json!({"x": 400, "y": 0, "width": 50, "height": 300, "name": "A narrow frame between two"})).unwrap();
    s.execute("frame.create", &json!({"x": 0, "y": -60, "width": 393, "height": 50, "name": "A bar just above Home — Light"})).unwrap();
    s.execute("select.none", &json!({})).unwrap();
    for zoom in [0.1, 0.22, 0.5, 1.0, 2.0] {
        for cx in [-900.0, 200.0, 700.0, 1150.0] {
            look(&mut u, cx, 200.0, zoom);
            check(&u, &format!("scenario {zoom} at {cx}"));
        }
    }
}

#[test]
fn pressing_a_moved_label_still_selects_its_frame() {
    let mut u = ridgeline();
    look(&mut u, 0.0, 450.0, 0.22);
    for (id, r) in areas(&u) {
        let c: Pos2 = r.center();
        u.app.session.execute("select.none", &json!({})).unwrap();
        u.button(c, true);
        u.button(c, false);
        assert_eq!(u.app.session.selection(), vec![id], "the label at {c:?}");
    }
}

#[test]
fn label_text_reads_on_both_canvases_and_both_default_pages() {
    let mut low = Vec::new();
    for (page, bg) in pages() {
        let ink = Ink::for_page(bg);
        for (role, fg) in [("title", ink.text), ("selected title", ink.accent), ("component title", ink.component)] {
            let r = contrast(fg, bg);
            if r < 4.5 {
                low.push(format!("{role} on the {page}: {r:.2}:1"));
            }
        }
    }
    assert!(low.is_empty(), "{low:#?}");
}

#[test]
fn labels_inside_a_section_read_on_its_fill_not_the_page() {
    // Ridgeline's page is #1E1E1E; the Components section is filled near-white. Its children's
    // labels (Landscape, Button, …) sit on that fill and take the light-page ink.
    let u = ridgeline();
    let doc = u.app.session.doc().unwrap();
    let painter = u.ctx.layer_painter(egui::LayerId::background());
    let t = crate::theme::Tokens::get(&u.ctx);
    let page = crate::frame_labels::backdrop(&painter, &t, doc, NodeId(349));
    for child in [14u64, 19, 28, 29, 55] {
        let bg = crate::frame_labels::backdrop(&painter, &t, doc, NodeId(child));
        assert_ne!(bg, page, "{child}: on the section's fill");
        let ink = Ink::for_page(bg);
        for (role, fg) in [("title", ink.text), ("component title", ink.component)] {
            assert!(contrast(fg, bg) >= 4.5, "{child}'s {role}: {:.2}:1", contrast(fg, bg));
        }
    }
    assert!(contrast(Ink::for_page(page).text, page) >= 4.5, "the section's own title, on the page");
}
