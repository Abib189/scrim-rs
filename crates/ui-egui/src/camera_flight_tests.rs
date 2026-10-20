//! D2 in the window: a Layers row click selects and flies the camera to the layer (320 ms,
//! eased), a jump with animations off, any input mid-flight stops it where the person put it, a
//! canvas click never moves the camera, a focus asked for before the canvas had its size lands
//! after it, Enter on a focused row focuses, hidden layers are focused with a toast, and a
//! multi-selection frames the union.

use egui::{Event, Modifiers, MouseWheelUnit, PointerButton, Pos2, pos2, vec2};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::layers_drag_tests::Ui;

fn id(v: Value) -> NodeId {
    figcraft_engine::params::id_of(&v["id"]).unwrap()
}

/// "Near" in view, "Far" well off to the right, the canvas sized by a first frame.
fn scene() -> (Ui, NodeId, NodeId) {
    let mut s = Session::new();
    let near = id(s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100, "name": "Near"})).unwrap());
    let far = id(s.execute("frame.create", &json!({"x": 6000, "y": 0, "width": 100, "height": 100, "name": "Far"})).unwrap());
    s.execute("select.none", &json!({})).unwrap();
    let mut u = Ui::new(s);
    for _ in 0..3 {
        u.frame(vec![]);
    }
    // A known camera: centred on Near at 100%.
    let ds = u.app.session.ds_mut().unwrap();
    ds.view.cx = 50.0;
    ds.view.cy = 50.0;
    ds.view.zoom = 1.0;
    u.frame(vec![]);
    (u, near, far)
}

fn view(u: &Ui) -> (f64, f64, f64) {
    let v = &u.app.session.ds().unwrap().view;
    (v.cx, v.cy, v.zoom)
}

fn click(u: &mut Ui, p: Pos2, modifiers: Modifiers) {
    u.frame_with(vec![Event::PointerMoved(p)], modifiers);
    u.frame_with(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers }], modifiers);
    u.frame_with(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers }], modifiers);
}

fn frames(u: &mut Ui, n: usize) {
    for _ in 0..n {
        u.frame(vec![]);
    }
}

#[test]
fn a_row_click_selects_and_flies_the_camera_there() {
    let (mut u, _, far) = scene();
    let from = view(&u);
    let row = u.row_at(far);
    click(&mut u, row, Modifiers::NONE);
    assert_eq!(u.app.session.selection(), [far]);
    frames(&mut u, 4);
    assert!(u.app.ui.camera.flying(), "it flies");
    let mid = view(&u);
    frames(&mut u, 40);
    let to = view(&u);
    assert!(!u.app.ui.camera.flying(), "and lands (320 ms)");
    assert!(to.0 > from.0 + 5000.0, "Far is in view: {to:?}");
    assert!(mid.0 > from.0 && mid.0 < to.0, "on the way, not a jump: {mid:?}");
    assert_eq!(to.2, from.2, "the zoom is kept");
    // The target is the engine's: the least move with a 32 px margin.
    let ds = u.app.session.ds().unwrap();
    assert!((6100.0 - (ds.view.cx + ds.view.vw / 2.0 - 32.0)).abs() < 1e-6, "Far's right edge at the margin: {}", ds.view.cx);
}

#[test]
fn with_animations_off_it_jumps() {
    let (mut u, _, far) = scene();
    u.app.run("pref.motion", json!({"motion": "off"}));
    let row = u.row_at(far);
    click(&mut u, row, Modifiers::NONE);
    assert!(!u.app.ui.camera.flying());
    assert!(view(&u).0 > 5000.0, "already there");
}

#[test]
fn a_scroll_mid_flight_stops_it_where_the_person_put_it() {
    let (mut u, _, far) = scene();
    let row = u.row_at(far);
    click(&mut u, row, Modifiers::NONE);
    frames(&mut u, 3);
    assert!(u.app.ui.camera.flying());
    // A wheel over the canvas.
    let over = pos2(700.0, 400.0);
    u.frame(vec![Event::PointerMoved(over)]);
    u.frame(vec![Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -120.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    }]);
    let after_scroll = view(&u);
    frames(&mut u, 40);
    assert!(!u.app.ui.camera.flying(), "the flight stopped");
    // The flight was moving x; the scroll (which egui eases over a few frames) moves y.
    assert_eq!(view(&u).0, after_scroll.0, "the flight no longer moves the camera: {:?}", view(&u));
    assert!(view(&u).0 < 5000.0, "it stayed short of the target: {:?}", view(&u));
    assert!(view(&u).1 != 50.0, "the scroll's own move stands");
}

#[test]
fn a_canvas_click_never_moves_the_camera() {
    let (mut u, near, _) = scene();
    let before = view(&u);
    // Near is at page (0,0)-(100,100): click its middle on screen.
    let (sx, sy) = {
        let ds = u.app.session.ds().unwrap();
        let (x, y) = ds.view.page_to_screen(50.0, 50.0);
        let r = u.app.canvas.rect;
        (r.min.x + x as f32, r.min.y + y as f32)
    };
    click(&mut u, pos2(sx, sy), Modifiers::NONE);
    frames(&mut u, 30);
    assert_eq!(u.app.session.selection(), [near], "the click selected");
    assert_eq!(view(&u), before, "and the camera stayed");
    assert!(!u.app.ui.camera.flying());
}

#[test]
fn a_focus_before_the_canvas_has_its_size_lands_after_it() {
    let mut s = Session::new();
    s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100, "name": "Near"})).unwrap();
    let far = id(s.execute("frame.create", &json!({"x": 6000, "y": 3000, "width": 100, "height": 100, "name": "Far"})).unwrap());
    let mut u = Ui::new(s);
    // Before the first frame: the canvas hasn't its size, and its first sizing re-fits the page.
    u.app.run("camera.focus-layer", json!({"id": far}));
    frames(&mut u, 3);
    let ds = u.app.session.ds().unwrap();
    let (half_w, half_h) = (ds.view.vw / 2.0 / ds.view.zoom, ds.view.vh / 2.0 / ds.view.zoom);
    let shown = |x: f64, y: f64| (x - ds.view.cx).abs() <= half_w && (y - ds.view.cy).abs() <= half_h;
    assert!(shown(6050.0, 3050.0), "Far is in view after the first sizing: {:?}", (ds.view.cx, ds.view.cy, ds.view.zoom));
}

#[test]
fn enter_on_a_focused_row_focuses_its_layer() {
    let (mut u, near, far) = scene();
    u.app.run("select.set", json!({"ids": [near]}));
    // F6 to the Layers region: the selected row takes the keyboard.
    for _ in 0..4 {
        if u.app.ui.nav.region == crate::a11y::Region::Layers {
            break;
        }
        u.frame(vec![
            Event::Key { key: egui::Key::F6, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
            Event::Key { key: egui::Key::F6, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
        ]);
        frames(&mut u, 1);
    }
    // Far is listed above Near (front first): Up, then Enter.
    for key in [egui::Key::ArrowUp, egui::Key::Enter] {
        u.frame(vec![
            Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
            Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::NONE },
        ]);
        frames(&mut u, 2);
    }
    frames(&mut u, 40);
    assert_eq!(u.app.session.selection(), [far]);
    assert!(view(&u).0 > 5000.0, "Enter focused Far: {:?}", view(&u));
}

#[test]
fn a_hidden_layer_is_focused_with_a_toast() {
    let (mut u, _, far) = scene();
    u.app.run("node.set", json!({"id": far, "props": {"visible": false}}));
    let row = u.row_at(far);
    click(&mut u, row, Modifiers::NONE);
    frames(&mut u, 40);
    assert!(view(&u).0 > 5000.0, "focused anyway");
    assert_eq!(u.app.ui.toast.as_ref().map(|t| t.0.as_str()), Some("“Far” is hidden"));
}

#[test]
fn a_multi_selection_frames_the_union() {
    let (mut u, near, far) = scene();
    let row = u.row_at(near);
    click(&mut u, row, Modifiers::NONE);
    frames(&mut u, 40);
    let row = u.row_at(far);
    click(&mut u, row, Modifiers::SHIFT);
    frames(&mut u, 40);
    assert_eq!(u.app.session.selection().len(), 2);
    // The union (0,0)-(6100,100) is wider than the view: its left edge at the margin.
    let ds = u.app.session.ds().unwrap();
    assert!((ds.view.cx - ds.view.vw / 2.0 + 32.0).abs() < 1e-6, "left edge at the margin: {}", ds.view.cx);
}
