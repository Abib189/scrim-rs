//! D1: Collapse all layers, from the Layers header's button (and ⌥L). Ported from the old Scrim's
//! `shell.test.ts` › collapse-all and `layersVirtual.dom.test.tsx` where they still apply: it leaves
//! only the top-level rows, an empty document is fine, and a partial fold folds the rest. It is
//! idempotent (the old button toggled; see D0), keeps the selection, leaves an active filter's list
//! alone, and opening a row by hand afterwards opens only that row.

use egui::accesskit::{Node, Role};
use egui::{Event, Modifiers, PointerButton, pos2};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::layers_drag_tests::Ui;

fn id(v: Value) -> NodeId {
    figcraft_engine::params::id_of(&v["id"]).unwrap()
}

fn tree(u: &mut Ui) -> Vec<Node> {
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    let mut out = u.frame_output(vec![]);
    out.textures_delta.clear();
    out.platform_output.accesskit_update.expect("an AccessKit update").nodes.into_iter().map(|(_, n)| n).collect()
}

/// The layer rows listed now, top to bottom.
fn rows(u: &mut Ui) -> Vec<String> {
    let mut r: Vec<(f64, String)> =
        tree(u).iter().filter(|n| n.role() == Role::TreeItem).filter_map(|n| Some((n.bounds()?.y0, n.label()?.to_string()))).collect();
    r.sort_by(|a, b| a.0.total_cmp(&b.0));
    r.into_iter().map(|(_, l)| l).collect()
}

fn click_named(u: &mut Ui, label: &str) {
    let nodes = tree(u);
    let n = nodes.iter().find(|n| n.label() == Some(label)).unwrap_or_else(|| panic!("no {label:?}"));
    let b = n.bounds().unwrap();
    let p = pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
    u.frame(vec![Event::PointerMoved(p)]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    u.frame(vec![]);
}

/// Two top-level frames, Poster and Kiln, each holding a frame with a rectangle in it, every row open.
fn nested() -> (Ui, [NodeId; 4]) {
    let mut s = Session::new();
    let mut ids = Vec::new();
    for (i, name) in ["Poster", "Kiln"].iter().enumerate() {
        let top = id(s.execute("frame.create", &json!({"x": 400 * i, "y": 0, "width": 300, "height": 300, "name": name})).unwrap());
        let inner = id(s
            .execute("frame.create", &json!({"x": 10, "y": 10, "width": 200, "height": 200, "parent": top, "name": format!("{name} body")}))
            .unwrap());
        let r = id(s.execute("shape.rectangle", &json!({"x": 5, "y": 5, "width": 50, "height": 50, "parent": inner})).unwrap());
        s.execute("object.rename", &json!({"ids": [r], "name": format!("{name} swatch")})).unwrap();
        ids.extend([top, inner]);
    }
    s.execute("select.none", &json!({})).unwrap();
    let mut u = Ui::new(s);
    u.app.ui.expanded.extend(ids.iter().copied());
    u.frame(vec![]);
    (u, [ids[0], ids[1], ids[2], ids[3]])
}

#[test]
fn the_button_collapses_every_row_to_the_top_level_and_is_idempotent() {
    let (mut u, _) = nested();
    assert_eq!(rows(&mut u).len(), 6, "every row open first: {:?}", rows(&mut u));
    let canvas_before = u.app.session.ds().unwrap().view.clone();
    click_named(&mut u, "Collapse all layers");
    assert_eq!(rows(&mut u), ["Kiln", "Poster"], "only the top-level rows (front first)");
    assert!(u.app.ui.expanded.is_empty());
    click_named(&mut u, "Collapse all layers");
    assert_eq!(rows(&mut u), ["Kiln", "Poster"], "a second press changes nothing");
    let ds = u.app.session.ds().unwrap();
    assert_eq!((ds.view.cx, ds.view.cy, ds.view.zoom), (canvas_before.cx, canvas_before.cy, canvas_before.zoom), "the camera doesn't move");
}

#[test]
fn the_selection_survives_and_a_partial_fold_folds_the_rest() {
    let (mut u, [poster, poster_body, ..]) = nested();
    u.app.run("select.set", json!({"ids": [poster_body]}));
    u.frame(vec![]);
    u.app.ui.expanded.remove(&poster);
    u.frame(vec![]);
    assert_eq!(rows(&mut u).len(), 4, "Poster folded by hand, Kiln open");
    u.app.run("layers.collapse-all", json!({}));
    assert_eq!(rows(&mut u), ["Kiln", "Poster"]);
    assert_eq!(u.app.session.selection(), [poster_body], "the selection stays (its row isn't listed)");
}

#[test]
fn an_empty_document_and_nothing_selected_are_fine() {
    let mut u = Ui::new(Session::new());
    u.frame(vec![]);
    u.app.run("layers.collapse-all", json!({}));
    assert!(rows(&mut u).is_empty());
    let (mut u, _) = nested();
    assert!(u.app.session.selection().is_empty());
    u.app.run("layers.collapse-all", json!({}));
    assert_eq!(rows(&mut u).len(), 2);
}

#[test]
fn an_active_filter_keeps_its_list_and_the_collapse_shows_after() {
    let (mut u, _) = nested();
    u.app.ui.layer_search = true;
    u.app.ui.layer_filter = "swatch".into();
    u.frame(vec![]);
    let filtered = rows(&mut u);
    assert!(filtered.contains(&"Poster swatch".to_string()) && filtered.contains(&"Kiln swatch".to_string()), "{filtered:?}");
    u.app.run("layers.collapse-all", json!({}));
    assert_eq!(rows(&mut u), filtered, "the filter lists matches from the whole page, collapsed or not");
    u.app.ui.layer_filter.clear();
    u.app.ui.layer_search = false;
    assert_eq!(rows(&mut u), ["Kiln", "Poster"], "cleared: the collapsed tree");
}

#[test]
fn opening_a_row_by_hand_afterwards_opens_only_that_row() {
    let (mut u, [poster, ..]) = nested();
    u.app.run("layers.collapse-all", json!({}));
    u.app.ui.expanded.insert(poster);
    assert_eq!(rows(&mut u), ["Kiln", "Poster", "Poster body"], "Poster's children, not its grandchildren");
}

#[test]
fn the_button_is_named_keyed_and_reachable_with_tab() {
    let (mut u, _) = nested();
    let nodes = tree(&mut u);
    let b = nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some("Collapse all layers")).expect("the button");
    assert!(b.keyboard_shortcut().is_some_and(|k| k.contains('L')), "its key: {:?}", b.keyboard_shortcut());
    // Into the Layers region (F6), then Tab within it.
    for _ in 0..4 {
        if u.app.ui.nav.region == crate::a11y::Region::Layers {
            break;
        }
        u.frame(vec![
            Event::Key { key: egui::Key::F6, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
            Event::Key { key: egui::Key::F6, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
        ]);
        u.frame(vec![]);
    }
    assert_eq!(u.app.ui.nav.region, crate::a11y::Region::Layers);
    let mut seen = Vec::new();
    for _ in 0..40 {
        u.frame(vec![
            Event::Key { key: egui::Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
            Event::Key { key: egui::Key::Tab, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
        ]);
        u.frame(vec![]);
        let name = crate::a11y::focused_name(&u.ctx);
        seen.push(name.clone());
        if name == "Collapse all layers" {
            u.frame(vec![
                Event::Key { key: egui::Key::Space, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
                Event::Key { key: egui::Key::Space, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
            ]);
            u.frame(vec![]);
            assert!(u.app.ui.expanded.is_empty(), "Space presses it");
            return;
        }
    }
    panic!("Tab never reached it: {seen:?}");
}
