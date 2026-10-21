//! I3, follow: an agent's edit off screen flies the camera there; work already in view moves it the
//! least (or not at all); a burst is one smooth move, at most one new flight per 400 ms; the
//! person's edit never moves it; a scroll mid-flight cancels and pauses; Resume flies to the latest
//! work; another page is shown first; Animations off jumps; nothing moves before the canvas has its
//! real size; a move waits while the pointer is down; an agent's own camera move doesn't pause;
//! big batches frame the artboard; the AI tab says what it follows; the command answers the state.

use egui::{Event, Modifiers, MouseWheelUnit, PointerButton, Pos2, pos2, vec2};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::{Value, json};

use super::{FLIGHT_EVERY, Landed, target};
use crate::layers_drag_tests::Ui;

pub(crate) fn id(v: &Value) -> NodeId {
    figcraft_engine::params::id_of(&v["id"]).unwrap()
}

/// Run `cmd` as an agent over the app's MCP endpoint route (the control queue's handler).
pub(crate) fn agent(u: &mut Ui, cmd: &str, params: Value) -> Value {
    let ctx = u.ctx.clone();
    crate::control::handle_route(&mut u.app, &ctx, "mcp (http)", "engine.execute", &json!({"command": cmd, "params": params})).unwrap()
}

pub(crate) fn view(u: &Ui) -> (f64, f64, f64) {
    let v = &u.app.session.ds().unwrap().view;
    (v.cx, v.cy, v.zoom)
}

pub(crate) fn click(u: &mut Ui, p: Pos2) {
    u.frame(vec![Event::PointerMoved(p)]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
}

pub(crate) fn frames(u: &mut Ui, n: usize) {
    for _ in 0..n {
        u.frame(vec![]);
    }
}

/// A frame "Home" at the origin with the camera on it at 100%, the canvas sized, an MCP client
/// named like Claude's connected.
pub(crate) fn scene() -> (Ui, NodeId) {
    let mut s = Session::new();
    let home = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 200, "height": 200, "name": "Home"})).unwrap());
    s.execute("select.none", &json!({})).unwrap();
    s.agents.mcp.clients.push(("claude-code".into(), "2.0".into(), 0));
    let mut u = Ui::new(s);
    frames(&mut u, 3);
    let ds = u.app.session.ds_mut().unwrap();
    ds.view.cx = 100.0;
    ds.view.cy = 100.0;
    ds.view.zoom = 1.0;
    frames(&mut u, 2);
    (u, home)
}

/// The agent puts a frame "Poster" far to the right.
pub(crate) fn poster(u: &mut Ui, x: f64) -> NodeId {
    id(&agent(u, "frame.create", json!({"x": x, "y": 0, "width": 400, "height": 300, "name": "Poster"})))
}

#[test]
fn an_agent_edit_off_screen_flies_the_camera_there() {
    let (mut u, _) = scene();
    let from = view(&u);
    let p = poster(&mut u, 8000.0);
    frames(&mut u, 3);
    assert!(u.app.ui.camera.flying(), "a flight, not a jump");
    let mid = view(&u);
    frames(&mut u, 40);
    let to = view(&u);
    assert!(mid.0 > from.0 && mid.0 < to.0, "on the way: {mid:?}");
    let b = u.app.session.doc().unwrap().world_bounds(p).unwrap();
    let ds = u.app.session.ds().unwrap();
    let (x0, x1) = (ds.view.cx - ds.view.vw / 2.0 / ds.view.zoom, ds.view.cx + ds.view.vw / 2.0 / ds.view.zoom);
    assert!(b.x0 >= x0 && b.x1 <= x1, "Poster in view: {b:?} in {x0}..{x1}");
    assert_eq!(to.2, from.2, "it fits at 100%: the zoom is kept");
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following");
}

#[test]
fn work_already_in_view_doesnt_move_the_camera_and_work_just_out_moves_it_the_least() {
    let (mut u, home) = scene();
    let before = view(&u);
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    frames(&mut u, 30);
    assert_eq!(view(&u), before, "in view: no move");
    // Just past the right margin: moved by about that much, not centred.
    let ds = u.app.session.ds().unwrap();
    let right = ds.view.cx + ds.view.vw / 2.0 - 32.0;
    agent(&mut u, "shape.rectangle", json!({"x": right + 10.0, "y": 50, "width": 20, "height": 20}));
    frames(&mut u, 40);
    let after = view(&u);
    assert!((after.0 - before.0 - 30.0).abs() < 1.0, "the least move: {after:?} from {before:?}");
    assert_eq!(after.1, before.1);
}

#[test]
fn a_burst_is_one_smooth_move_at_most_one_flight_per_400_ms() {
    let (mut u, _) = scene();
    let p = poster(&mut u, 8000.0);
    let mut flights = Vec::new();
    let mut xs = Vec::new();
    // Sixteen edits, one every other frame (about 0.53 s in all).
    for i in 0..16 {
        agent(&mut u, "shape.rectangle", json!({"x": 10 + 20 * i, "y": 10, "width": 10, "height": 10, "parent": p}));
        for _ in 0..2 {
            u.frame(vec![]);
            if let Some(t) = u.app.ui.agent_view.follow.last_flight
                && flights.last() != Some(&t)
            {
                flights.push(t);
            }
            xs.push(view(&u).0);
        }
    }
    frames(&mut u, 40);
    assert!(flights.len() <= 3, "at most one flight per 400 ms over the burst: {flights:?}");
    assert!(flights.windows(2).all(|w| w[1] - w[0] >= FLIGHT_EVERY - 1e-9), "{flights:?}");
    assert!(xs.windows(2).all(|w| w[1] >= w[0] - 1e-6), "never back and forth: {xs:?}");
}

#[test]
fn the_persons_edit_never_moves_the_camera_and_pauses_following() {
    let (mut u, home) = scene();
    let before = view(&u);
    // Before any agent work, the person's edits are just theirs: nothing to pause.
    u.app.run("frame.create", json!({"x": 9000, "y": 0, "width": 100, "height": 100, "name": "Mine"}));
    frames(&mut u, 30);
    assert_eq!(view(&u), before, "the person's own edit: no move");
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following", "nothing followed yet");
    // An agent works (in view), then the person edits: paused.
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    frames(&mut u, 30);
    u.app.run("frame.create", json!({"x": 9500, "y": 0, "width": 100, "height": 100, "name": "Mine 2"}));
    frames(&mut u, 30);
    assert_eq!(view(&u), before, "the person's own edit: no move");
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "paused");
    // Paused: an agent's edit off screen doesn't move it either.
    poster(&mut u, 8000.0);
    frames(&mut u, 30);
    assert_eq!(view(&u), before);
}

#[test]
fn a_scroll_mid_flight_cancels_and_pauses_and_resume_flies_to_the_latest_work() {
    let (mut u, _) = scene();
    poster(&mut u, 8000.0);
    frames(&mut u, 4);
    assert!(u.app.ui.camera.flying());
    let over = u.app.canvas.rect.center();
    u.frame(vec![Event::PointerMoved(over)]);
    u.frame(vec![Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -120.0),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    }]);
    frames(&mut u, 30);
    assert!(!u.app.ui.camera.flying(), "cancelled");
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "paused");
    let stopped = view(&u);
    assert!(stopped.0 < 7000.0, "it stayed where the person put it: {stopped:?}");
    // More work lands while paused: nothing moves.
    let second = id(&agent(&mut u, "frame.create", json!({"x": 12000, "y": 0, "width": 200, "height": 200, "name": "Second"})));
    frames(&mut u, 30);
    assert_eq!(view(&u).0, stopped.0);
    // The chip is shown; a click on it resumes and flies to the latest work.
    let chip = u.app.ui.agent_view.follow.chip.expect("the paused chip");
    click(&mut u, chip.center());
    u.frame(vec![Event::PointerMoved(pos2(5.0, 5.0))]);
    frames(&mut u, 40);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following");
    let b = u.app.session.doc().unwrap().world_bounds(second).unwrap();
    let ds = u.app.session.ds().unwrap();
    assert!((ds.view.cx - b.center().x).abs() < ds.view.vw / ds.view.zoom, "at the latest work: {} vs {:?}", ds.view.cx, b);
}

#[test]
fn work_on_another_page_shows_that_page_first() {
    let (mut u, _) = scene();
    let first = u.app.session.ds().unwrap().page;
    agent(&mut u, "page.add", json!({"name": "Two"}));
    agent(&mut u, "page.select", json!({"index": 0}));
    frames(&mut u, 3);
    assert_eq!(u.app.session.ds().unwrap().page, first);
    // The agent edits page two without showing it (through a layer id there).
    let two = u.app.session.doc().unwrap().pages[1];
    let r = agent(&mut u, "shape.rectangle", json!({"x": 50, "y": 50, "width": 30, "height": 30, "parent": two}));
    frames(&mut u, 30);
    assert_eq!(u.app.session.ds().unwrap().page, two, "page two is shown: {r}");
}

#[test]
fn with_animations_off_it_jumps() {
    let (mut u, _) = scene();
    u.app.run("pref.motion", json!({"motion": "off"}));
    poster(&mut u, 8000.0);
    frames(&mut u, 2);
    assert!(!u.app.ui.camera.flying(), "a jump cut");
    assert!(view(&u).0 > 7000.0, "already there");
}

#[test]
fn nothing_moves_before_the_canvas_has_its_real_size() {
    let mut s = Session::new();
    s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100})).unwrap();
    let mut u = Ui::new(s);
    // Unsized: the view's size doesn't match the canvas yet.
    u.app.session.ds_mut().unwrap().view.vw = 1.0;
    let ctx = u.ctx.clone();
    super::before_canvas(&mut u.app, &ctx);
    poster(&mut u, 8000.0);
    super::before_canvas(&mut u.app, &ctx);
    assert!(u.app.ui.agent_view.follow.last_flight.is_none(), "no flight before the canvas has its size");
    // Once sized (the next real frames), it goes there.
    frames(&mut u, 40);
    assert!(view(&u).0 > 7000.0, "after the first sizing: {:?}", view(&u));
}

#[test]
fn a_move_waits_while_the_pointer_is_down_then_goes() {
    let (mut u, _) = scene();
    // The pointer held down over the right panel (a slider drag, say): not the canvas.
    let panel = pos2(u.app.canvas.rect.max.x + 40.0, 300.0);
    u.frame(vec![Event::PointerMoved(panel)]);
    u.frame(vec![Event::PointerButton { pos: panel, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    let before = view(&u);
    poster(&mut u, 8000.0);
    frames(&mut u, 20);
    assert_eq!(view(&u), before, "held: no move");
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following", "and not paused");
    u.frame(vec![Event::PointerButton { pos: panel, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    frames(&mut u, 40);
    assert!(view(&u).0 > 7000.0, "released: it goes");
}

#[test]
fn an_agents_own_camera_move_doesnt_pause_following() {
    let (mut u, _) = scene();
    agent(&mut u, "view.set", json!({"cx": 3000, "cy": 0, "zoom": 1}));
    frames(&mut u, 5);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following");
    poster(&mut u, 8000.0);
    frames(&mut u, 40);
    assert!(view(&u).0 > 7000.0);
}

#[test]
fn a_canvas_click_pauses_but_a_click_on_the_toolbar_doesnt() {
    let (mut u, home) = scene();
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 20, "height": 20, "parent": home}));
    frames(&mut u, 30);
    let tool = pos2(u.app.canvas.rect.center().x, u.app.canvas.rect.max.y - 30.0);
    click(&mut u, tool);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following", "the toolbar isn't the canvas");
    let empty = pos2(u.app.canvas.rect.center().x + 200.0, u.app.canvas.rect.center().y + 150.0);
    click(&mut u, empty);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "paused");
}

#[test]
fn big_batches_frame_the_artboard_and_a_new_top_level_frame_is_framed_whole() {
    let mut s = Session::new();
    let art = id(&s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 1000, "height": 1000, "name": "Art"})).unwrap());
    let kids: Vec<NodeId> =
        (0..70).map(|i| id(&s.execute("shape.rectangle", &json!({"x": i * 10, "y": 5, "width": 5, "height": 5, "parent": art})).unwrap())).collect();
    let doc = s.doc().unwrap().clone();
    let many = [Landed { agent: true, changed: kids.clone(), ..Landed::default() }];
    assert_eq!(target(&doc, &many).unwrap().1.into_iter().collect::<Vec<_>>(), [art], "70 leaves: the artboard");
    let few = [Landed { agent: true, changed: kids[..3].to_vec(), ..Landed::default() }];
    assert_eq!(target(&doc, &few).unwrap().1.len(), 3, "a few: the layers themselves");
    // Created with its children: the new frame whole (the parent isn't dropped as "only a parent").
    let made = [Landed { agent: true, created: [&[art][..], &kids[..3]].concat(), ..Landed::default() }];
    assert!(target(&doc, &made).unwrap().1.contains(&art));
    // The person's edits are never a target.
    let mine = [Landed { agent: false, created: vec![art], ..Landed::default() }];
    assert!(target(&doc, &mine).is_none());
}

#[test]
fn the_ai_tab_says_who_and_what_it_follows_and_the_command_answers_the_state() {
    let (mut u, _) = scene();
    poster(&mut u, 8000.0);
    frames(&mut u, 5);
    assert_eq!(super::status(&u.app).as_deref(), Some("Following Claude · Poster"));
    let r = agent(&mut u, "camera.follow-agents", json!({"on": false}));
    assert_eq!(r["state"], "off", "{r}");
    frames(&mut u, 2);
    assert_eq!(super::status(&u.app), None);
    let r = agent(&mut u, "camera.follow-agents", json!({}));
    assert_eq!(r["state"], "following", "{r}");
}

#[test]
fn an_activity_row_click_selects_and_focuses_its_layers() {
    let (mut u, _) = scene();
    u.app.run("camera.follow-agents", json!({"on": false}));
    let p = poster(&mut u, 8000.0);
    frames(&mut u, 3);
    let before = view(&u);
    let feed = u.app.run("history.changes", json!({})).unwrap();
    let revision = feed["changes"].as_array().unwrap().last().unwrap()["revision"].as_u64().unwrap();
    u.app.run("ai.selectChange", json!({"revision": revision}));
    crate::camera_flight::focus_selection(&mut u.app);
    frames(&mut u, 40);
    assert_eq!(u.app.session.selection(), [p]);
    assert!(view(&u).0 > before.0 + 5000.0, "focused: {:?}", view(&u));
}
