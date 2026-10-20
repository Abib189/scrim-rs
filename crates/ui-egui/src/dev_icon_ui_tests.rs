//! E1 in the window (egui headless): each status is an icon after its frame's title, outside the
//! frame, named for screen readers and in a tooltip; a long title ends before it; a narrow frame
//! drops the icon, then the label; a section's title and icon sit above it; pressing the icon
//! selects its own frame.

use egui::accesskit::{Node, Role};
use egui::{Event, Pos2, Rect, pos2};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::layers_drag_tests::Ui;

fn id(v: Value) -> NodeId {
    figcraft_engine::params::id_of(&v["id"]).unwrap()
}

struct Scene {
    u: Ui,
    ready: NodeId,
    long: NodeId,
    changed: NodeId,
    narrow: NodeId,
    tiny: NodeId,
    section: NodeId,
}

const LONG: &str = "Checkout — the long variant with every payment method, saved card and promo code shown";

/// At 50%: Ready (300 wide), a long-titled Completed frame, a Changed frame, a 60-wide and a
/// 20-wide Ready frame, and a Ready section below them.
fn scene() -> Scene {
    let mut s = Session::new();
    let frame = |s: &mut Session, x: f64, w: f64, name: &str| {
        id(s.execute("frame.create", &json!({"x": x, "y": 0, "width": w, "height": 200, "name": name})).unwrap())
    };
    let ready = frame(&mut s, 0.0, 300.0, "Ready");
    let long = frame(&mut s, 360.0, 300.0, LONG);
    let changed = frame(&mut s, 720.0, 300.0, "Changed");
    let narrow = frame(&mut s, 1080.0, 60.0, "Narrow frame");
    let tiny = frame(&mut s, 1180.0, 20.0, "Tiny");
    let section = id(s.execute("node.create", &json!({"type": "section", "x": 0, "y": 400, "width": 600, "height": 300, "name": "Flows"})).unwrap());
    let child = id(s.execute("shape.rectangle", &json!({"x": 20, "y": 20, "width": 40, "height": 40, "parent": changed})).unwrap());
    for (f, st) in [
        (ready, "readyForDev"),
        (long, "completed"),
        (changed, "readyForDev"),
        (narrow, "readyForDev"),
        (tiny, "readyForDev"),
        (section, "readyForDev"),
    ] {
        s.execute("node.set", &json!({"id": f, "props": {"devStatus": st}})).unwrap();
    }
    // Changed: its contents changed after it was marked.
    s.execute("node.set", &json!({"id": child, "props": {"width": 60}})).unwrap();
    s.execute("select.none", &json!({})).unwrap();
    let mut u = Ui::new(s);
    let ds = u.app.session.ds_mut().unwrap();
    (ds.view.cx, ds.view.cy, ds.view.zoom) = (640.0, 300.0, 0.5);
    u.frame(vec![]);
    Scene { u, ready, long, changed, narrow, tiny, section }
}

fn tree(u: &mut Ui) -> Vec<Node> {
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    let out = u.frame_output(vec![]);
    out.platform_output.accesskit_update.expect("an AccessKit update").nodes.into_iter().map(|(_, n)| n).collect()
}

/// The status icons on screen: (status, rect).
fn icons(u: &mut Ui) -> Vec<(String, Rect)> {
    tree(u)
        .iter()
        .filter(|n| n.role() == Role::Image)
        .filter_map(|n| {
            let b = n.bounds()?;
            Some((n.label()?.to_string(), Rect::from_min_max(pos2(b.x0 as f32, b.y0 as f32), pos2(b.x1 as f32, b.y1 as f32))))
        })
        .filter(|(l, _)| ["Ready for dev", "Changed", "Completed"].contains(&l.as_str()))
        .collect()
}

/// Frame `f`'s label area on screen, if it has one.
fn label(u: &Ui, f: NodeId) -> Option<Rect> {
    let (_, q) = u.app.session.tool.labels.iter().find(|(id, _)| *id == f)?;
    Some(Rect::from_points(&q.map(|p| u.app.to_screen(p.x, p.y))))
}

/// Frame `f` on screen.
fn body(u: &Ui, f: NodeId) -> Rect {
    let b = u.app.session.doc().unwrap().world_bounds(f).unwrap();
    Rect::from_two_pos(u.app.to_screen(b.x0, b.y0), u.app.to_screen(b.x1, b.y1))
}

/// The icon inside `f`'s label.
fn icon_of(u: &mut Ui, f: NodeId) -> Option<(String, Rect)> {
    let l = label(u, f)?;
    icons(u).into_iter().find(|(_, r)| l.expand(0.5).contains_rect(*r))
}

#[test]
fn each_status_is_an_icon_after_the_title_outside_the_frame() {
    let mut sc = scene();
    for (f, want) in [(sc.ready, "Ready for dev"), (sc.long, "Completed"), (sc.changed, "Changed"), (sc.section, "Ready for dev")] {
        let (got, icon) = icon_of(&mut sc.u, f).unwrap_or_else(|| panic!("{want}: no icon in {f:?}'s label"));
        assert_eq!(got, want);
        let (l, b) = (label(&sc.u, f).unwrap(), body(&sc.u, f));
        assert!((icon.width() - 14.0).abs() < 0.5 && (icon.height() - 14.0).abs() < 0.5, "14 px: {icon:?}");
        assert!(icon.max.y <= b.min.y, "{want}: above the frame, never on it: {icon:?} vs {b:?}");
        assert!(icon.max.x <= b.max.x + 0.5, "{want}: within the frame's width: {icon:?} vs {b:?}");
        assert!((icon.max.x - l.max.x).abs() < 0.5, "{want}: the label ends with it: {icon:?} in {l:?}");
        assert!(icon.min.x >= l.min.x + 20.0, "{want}: after a title: {icon:?} in {l:?}");
    }
}

#[test]
fn a_long_title_ends_before_the_icon_and_never_past_its_frame() {
    let mut sc = scene();
    let (l, b) = (label(&sc.u, sc.long).unwrap(), body(&sc.u, sc.long));
    assert!(l.max.x <= b.max.x + 0.5, "{l:?} vs {b:?}");
    let (_, icon) = icon_of(&mut sc.u, sc.long).expect("the icon stays");
    assert!(icon.min.x > l.max.x - 14.5, "the icon is the label's end, the title cut before it");
}

#[test]
fn a_narrow_frame_drops_the_icon_then_the_label() {
    let mut sc = scene();
    // 30 px on screen: room for a cut title, not for the icon too.
    let l = label(&sc.u, sc.narrow).expect("a 30 px frame keeps its title");
    assert!(l.width() <= 30.5, "{l:?}");
    assert!(icon_of(&mut sc.u, sc.narrow).is_none(), "no icon without room for it");
    // 10 px: nothing, and no icon drawn on its own anywhere near it.
    assert!(label(&sc.u, sc.tiny).is_none());
    let b = body(&sc.u, sc.tiny).expand2(egui::vec2(20.0, 30.0));
    assert!(icons(&mut sc.u).iter().all(|(_, r)| !b.intersects(*r)), "never a detached icon");
}

#[test]
fn a_sections_title_and_icon_sit_above_it() {
    let sc = scene();
    let (l, b) = (label(&sc.u, sc.section).unwrap(), body(&sc.u, sc.section));
    assert!(l.max.y <= b.min.y + 0.5 && l.min.y >= b.min.y - 20.0, "above the top edge: {l:?} vs {b:?}");
    assert!((l.min.x - b.min.x).abs() < 0.5, "from its left edge");
}

#[test]
fn the_icon_is_named_in_a_tooltip_and_pressing_it_selects_its_frame() {
    let mut sc = scene();
    let (_, icon) = icon_of(&mut sc.u, sc.ready).unwrap();
    let c: Pos2 = icon.center();
    for _ in 0..3 {
        sc.u.frame(vec![Event::PointerMoved(c)]);
    }
    let shown = tree(&mut sc.u).iter().filter(|n| n.label() == Some("Ready for dev")).count();
    assert!(shown >= 2, "the icon and its tooltip: {shown}");
    sc.u.button(c, true);
    sc.u.button(c, false);
    assert_eq!(sc.u.app.session.selection(), vec![sc.ready]);
}
