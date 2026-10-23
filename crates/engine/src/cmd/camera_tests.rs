//! D2: focusing layers. Ported from the old Scrim's `cameraNav.test.tsx` (revealCamera): a layer
//! already in view doesn't move the camera, one partly out moves it the least, one bigger than the
//! view gets its leading edge at the margin, and the zoom never changes. Then what scrim-rs adds:
//! the union of several layers, another page shown first, hidden layers focused and reported.

use figcraft_doc::NodeId;
use figcraft_doc::geom::Rect;
use serde_json::{Value, json};

use super::{FLY_REQUEST, REVEAL_MARGIN, reveal};
use crate::{Session, ViewState};

fn view(cx: f64, cy: f64, zoom: f64) -> ViewState {
    ViewState { cx, cy, zoom, vw: 1000.0, vh: 800.0, ..ViewState::default() }
}

fn id(v: Value) -> NodeId {
    crate::params::id_of(&v["id"]).unwrap()
}

#[test]
fn a_layer_in_view_doesnt_move_the_camera() {
    // The view at zoom 1 shows x -500…500, y -400…400; the margin leaves -468…468, -368…368.
    let v = view(0.0, 0.0, 1.0);
    assert_eq!(reveal(&v, Rect::new(-100.0, -100.0, 100.0, 100.0)), (0.0, 0.0));
    assert_eq!(reveal(&v, Rect::new(-468.0, -368.0, 468.0, 368.0)), (0.0, 0.0), "exactly at the margin");
}

#[test]
fn a_layer_partly_out_moves_the_camera_the_least_on_each_axis() {
    let v = view(0.0, 0.0, 1.0);
    // 32 px past the right margin: the camera moves 32 px right, not to centre it.
    assert_eq!(reveal(&v, Rect::new(400.0, 0.0, 500.0, 100.0)), (32.0, 0.0));
    // Above the top: up by the amount it sticks out, x untouched.
    assert_eq!(reveal(&v, Rect::new(0.0, -500.0, 100.0, -400.0)), (0.0, -132.0));
    // At zoom 2 the margin is 16 page units.
    let v2 = view(0.0, 0.0, 2.0);
    let (cx, _) = reveal(&v2, Rect::new(300.0, 0.0, 340.0, 10.0));
    assert!((cx - (340.0 - (250.0 - REVEAL_MARGIN / 2.0))).abs() < 1e-9, "{cx}");
}

#[test]
fn a_layer_bigger_than_the_view_gets_its_leading_edge_at_the_margin() {
    let v = view(0.0, 0.0, 1.0);
    // 2000 wide, 100 tall: x so that its left edge sits at the margin; y the least move.
    let (cx, cy) = reveal(&v, Rect::new(1000.0, 0.0, 3000.0, 100.0));
    assert_eq!(cx, 1000.0 + 500.0 - REVEAL_MARGIN);
    assert_eq!(cy, 0.0);
}

/// Two frames far apart on page 1, a third on page 2; the canvas 1000 × 800.
fn session() -> (Session, [NodeId; 3]) {
    let mut s = Session::new();
    let a = id(s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 100, "height": 100, "name": "A"})).unwrap());
    let b = id(s.execute("frame.create", &json!({"x": 3000, "y": 2000, "width": 100, "height": 100, "name": "B"})).unwrap());
    s.execute("page.add", &json!({})).unwrap();
    let c = id(s.execute("frame.create", &json!({"x": 50, "y": 50, "width": 10, "height": 10, "name": "C"})).unwrap());
    s.execute("page.select", &json!({"index": 0})).unwrap();
    let ds = s.ds_mut().unwrap();
    ds.view = view(50.0, 50.0, 1.0);
    (s, [a, b, c])
}

#[test]
fn focusing_keeps_the_zoom_and_reveals_the_union_of_the_selection() {
    let (mut s, [a, b, _]) = session();
    let r = s.execute("camera.focus-layer", &json!({"id": a})).unwrap();
    assert_eq!(r["moved"], false, "A is in view already");
    s.ui_requests.clear();
    s.execute("select.set", &json!({"ids": [a, b]})).unwrap();
    let r = s.execute("camera.focus-layer", &json!({})).unwrap();
    assert_eq!(r["moved"], true);
    let v = &s.ds().unwrap().view;
    assert_eq!(v.zoom, 1.0, "the zoom is kept");
    // The union (0,0)–(3100,2100) is bigger than the view: its top-left corner goes to the margin.
    assert_eq!((v.cx, v.cy), (500.0 - REVEAL_MARGIN, 400.0 - REVEAL_MARGIN));
    let fly = s.ui_requests.iter().find(|q| q.kind == FLY_REQUEST).expect("the window animates");
    assert_eq!(fly.data["from"]["cx"], 50.0, "from where the camera was");
}

#[test]
fn another_page_is_shown_first_and_hidden_layers_are_focused_and_reported() {
    let (mut s, [a, _, c]) = session();
    let r = s.execute("camera.focus-layer", &json!({"id": c})).unwrap();
    assert_eq!(r["switched"], true);
    assert_eq!(s.page().unwrap(), crate::params::id_of(&r["page"]).unwrap(), "page 2 is shown");
    s.execute("page.select", &json!({"index": 0})).unwrap();
    s.execute("node.set", &json!({"id": a, "props": {"visible": false}})).unwrap();
    let r = s.execute("camera.focus-layer", &json!({"id": a})).unwrap();
    assert_eq!(r["hidden"], json!([a.0]), "focused, and reported hidden");
}

#[test]
fn nothing_to_focus_is_an_error_and_the_document_is_untouched() {
    let (mut s, _) = session();
    s.execute("select.none", &json!({})).unwrap();
    let doc = s.ds().unwrap().doc.clone();
    assert!(s.execute("camera.focus-layer", &json!({})).unwrap_err().to_string().contains("nothing to focus"));
    assert!(s.execute("camera.focus-layer", &json!({"id": 999_999})).is_err());
    assert!(std::sync::Arc::ptr_eq(&s.ds().unwrap().doc, &doc), "the camera is not the document");
}

/// Stage I: `fit` zooms out just enough when the layers don't fit, never in; without `fit` D2's
/// rule is unchanged.
#[test]
fn fit_zooms_out_only_when_the_layers_dont_fit() {
    let v = view(0.0, 0.0, 1.0);
    // Fits at 100%: the same least move as `reveal`, the zoom kept.
    assert_eq!(super::reveal_or_fit(&v, Rect::new(400.0, 0.0, 500.0, 100.0)), (32.0, 0.0, 1.0));
    // 3000 × 1000 doesn't fit in 1000 × 800: the zoom that fits it with the margin, centred on it.
    let (cx, cy, z) = super::reveal_or_fit(&v, Rect::new(0.0, 0.0, 3000.0, 1000.0));
    assert_eq!((cx, cy), (1500.0, 500.0));
    assert!((z - (1000.0 - 2.0 * REVEAL_MARGIN) / 3000.0).abs() < 1e-12, "{z}");
    // Out of view entirely: centred on it, not parked on the far edge.
    assert_eq!(super::reveal_or_fit(&v, Rect::new(5000.0, 3000.0, 5100.0, 3100.0)), (5050.0, 3050.0, 1.0));
    // A small layer at a low zoom isn't zoomed into.
    let low = view(0.0, 0.0, 0.1);
    assert_eq!(super::reveal_or_fit(&low, Rect::new(0.0, 0.0, 10.0, 10.0)).2, 0.1);
    // Through the command.
    let (mut s, [a, b, _]) = session();
    s.ds_mut().unwrap().view = view(0.0, 0.0, 4.0);
    let r = s.execute("camera.focus-layer", &json!({"ids": [a, b], "fit": true})).unwrap();
    assert!(r["view"]["zoom"].as_f64().unwrap() < 4.0, "zoomed out to fit: {r}");
    s.ds_mut().unwrap().view = view(0.0, 0.0, 4.0);
    let r = s.execute("camera.focus-layer", &json!({"ids": [a, b]})).unwrap();
    assert_eq!(r["view"]["zoom"], 4.0, "without fit, D2: the zoom kept");
}

/// `camera.follow-agents`: on by default; toggles; `on: true` while paused resumes; the state is
/// returned (the MCP route's answer); saved in the settings.
#[test]
fn follow_agents_turns_on_off_and_resumes() {
    let mut s = Session::new();
    assert!(s.prefs.settings.follow_agents, "on by default");
    assert_eq!(super::follow_state(&s), "following");
    let r = s.execute("camera.follow-agents", &json!({})).unwrap();
    assert_eq!(r, json!({"on": false, "state": "off"}), "toggled off");
    let r = s.execute("camera.follow-agents", &json!({})).unwrap();
    assert_eq!(r, json!({"on": true, "state": "following"}));
    s.agents.follow_paused = true;
    assert_eq!(super::follow_state(&s), "paused");
    let r = s.execute("camera.follow-agents", &json!({})).unwrap();
    assert_eq!(r["state"], "following", "a toggle while paused resumes: {r}");
    s.agents.follow_paused = true;
    let r = s.execute("camera.follow-agents", &json!({"on": true})).unwrap();
    assert_eq!(r["state"], "following");
    let r = s.execute("camera.follow-agents", &json!({"on": false})).unwrap();
    assert_eq!(r["state"], "off");
    // Saved: the settings round-trip keeps it, and a file without it reads on.
    let json = serde_json::to_value(&s.prefs.settings).unwrap();
    assert_eq!(json["follow_agents"], false);
    let back: crate::settings::Settings = serde_json::from_value(json).unwrap();
    assert!(!back.sanitized().follow_agents);
    let old: crate::settings::Settings = serde_json::from_value(json!({"settings_version": 2})).unwrap();
    let old = old.sanitized();
    assert!(old.follow_agents, "a file from before follow reads on");
    assert_eq!(old.settings_version, crate::settings::SETTINGS_VERSION);
}

/// Fix V2: only the person turns following on or off. An agent, by any route and any form of the
/// command (no `on`, `on: false`, `on: true`, inside `command.batch`), is refused in plain words,
/// the setting stays as it was, and Settings ▸ Diagnostics notes the refusal: the route in fixed
/// words and the session, nothing the agent sent.
#[test]
fn an_agent_cant_change_following_and_the_refusal_is_noted() {
    let mut s = Session::new();
    for (route, said) in [
        ("mcp (http)", "over MCP, session 3f1520b6…"),
        ("cli", "from the CLI"),
        ("control channel", "over the control channel"),
        ("plugin Evil \"name\"", "from a plugin"),
    ] {
        s.set_author(crate::agents::Author::agent(route));
        s.agents.session = route.starts_with("mcp").then(|| "3f1520b6-77aa-4bcd".to_string());
        for params in [json!({}), json!({"on": false}), json!({"on": true})] {
            let e = s.execute("camera.follow-agents", &params).expect_err("refused");
            assert!(e.to_string().contains("only the person turns following on or off"), "{route}: {e}");
            assert!(s.prefs.settings.follow_agents, "{route} {params}: still on");
        }
        let e = s
            .execute("command.batch", &json!({"commands": [{"command": "camera.follow-agents", "params": {}}]}))
            .expect_err("refused in a batch too");
        assert!(e.to_string().contains("only the person"), "{e}");
        assert!(s.prefs.settings.follow_agents);
        let last = s.prefs.settings.diagnostics.last().unwrap();
        assert_eq!((last.kind.as_str(), last.detail.clone()), ("follow", format!("follow setting: stays ON (refused: an agent asked, {said})")));
    }
    assert!(s.prefs.settings.diagnostics.iter().all(|e| !e.detail.contains("Evil")), "never a name the plugin gave");
    // The person's own change still works, off and on.
    s.set_author(crate::agents::Author::Person);
    assert_eq!(s.execute("camera.follow-agents", &json!({"on": false})).unwrap()["state"], "off");
    assert_eq!(s.execute("camera.follow-agents", &json!({})).unwrap()["state"], "following");
}

/// Fix V3: every change of Follow agents is noted in Settings ▸ Diagnostics with who made it: the
/// person's button, Settings toggle, resume or command (the palette, a menu), or a settings
/// migration. No line when nothing changed (a resume while on); an agent's attempt is the refusal's
/// line (V2).
#[test]
fn every_change_of_following_is_noted_with_its_origin() {
    let mut s = Session::new();
    let last = |s: &Session| s.prefs.settings.diagnostics.last().map(|e| (e.kind.clone(), e.detail.clone()));
    let line = |text: &str| Some(("follow".to_string(), text.to_string()));
    s.execute("camera.follow-agents", &json!({"on": false, "origin": "button"})).unwrap();
    assert_eq!(last(&s), line("follow setting: ON → OFF (made by the person's button)"));
    s.execute("camera.follow-agents", &json!({"on": true, "origin": "settings"})).unwrap();
    assert_eq!(last(&s), line("follow setting: OFF → ON (made by the person's Settings toggle)"));
    s.execute("camera.follow-agents", &json!({})).unwrap();
    assert_eq!(last(&s), line("follow setting: ON → OFF (made by the person's command)"), "the palette or a menu: no origin");
    s.execute("camera.follow-agents", &json!({"on": true, "origin": "resume"})).unwrap();
    assert_eq!(last(&s), line("follow setting: OFF → ON (made by the person's resume)"));
    s.execute("camera.follow-agents", &json!({"on": true, "origin": "made up"})).unwrap();
    let n = s.prefs.settings.diagnostics.len();
    // A resume while on changes nothing: no line.
    s.agents.follow_paused = true;
    s.execute("camera.follow-agents", &json!({"on": true, "origin": "resume"})).unwrap();
    assert_eq!(s.prefs.settings.diagnostics.len(), n, "a resume alone isn't a change of the setting");
    // A migration that changed it would say so.
    let mut settings = crate::settings::Settings { follow_agents: false, ..Default::default() };
    crate::settings::note_follow_migration(&mut settings, true);
    assert_eq!(settings.diagnostics.last().map(|e| e.detail.as_str()), Some("follow setting: ON → OFF (made by a settings migration)"));
    let mut same = crate::settings::Settings::default();
    crate::settings::note_follow_migration(&mut same, true);
    assert!(same.diagnostics.is_empty(), "no change, no line");
    // The real migrations never do (every version keeps following: apps/figcraft follow_setting_tests).
    let old: crate::settings::Settings = serde_json::from_value(json!({"settings_version": 3, "follow_agents": true})).unwrap();
    assert!(old.sanitized().diagnostics.is_empty());
    // An agent: the refusal's line, and nothing else.
    s.set_author(crate::agents::Author::agent("mcp (http)"));
    s.agents.session = None;
    assert!(s.execute("camera.follow-agents", &json!({"on": false, "origin": "button"})).is_err());
    assert_eq!(last(&s), line("follow setting: stays ON (refused: an agent asked, over MCP)"), "an agent can't pass for the button");
}

/// Fix W1: following is on wherever Scrim can start, from one constant: a new session (the web
/// build's, and the desktop app's before its settings load), the settings' default (a fresh
/// folder), and a settings file without the key. No reset path exists that could start elsewhere
/// (`toolbar.reset` resets the toolbar only).
#[test]
fn following_is_on_by_default_wherever_scrim_starts() {
    let s = Session::new();
    assert_eq!(s.prefs.settings.follow_agents, crate::settings::DEFAULT_FOLLOW_AGENTS, "the one constant");
    assert_eq!(super::follow_state(&s), "following", "a new session: the web build's start");
    assert!(crate::settings::Settings::default().follow_agents, "a fresh settings folder: on");
    let read: crate::settings::Settings = serde_json::from_value(json!({})).unwrap();
    assert!(read.follow_agents && !read.follow_chosen, "a file without the keys");
    let prefs: crate::Prefs = serde_json::from_value(json!({})).unwrap();
    assert!(prefs.settings.follow_agents);
}

/// Fix W1: the marker that the person chose following is written by the person's own change only:
/// never by a resume that changes nothing, an agent (refused), or a default.
#[test]
fn only_the_persons_change_marks_following_as_chosen() {
    let mut s = Session::new();
    assert!(!s.prefs.settings.follow_chosen, "not by the default");
    s.agents.follow_paused = true;
    s.execute("camera.follow-agents", &json!({"on": true, "origin": "resume"})).unwrap();
    assert!(!s.prefs.settings.follow_chosen, "a resume that changes nothing isn't a choice");
    s.set_author(crate::agents::Author::agent("mcp (http)"));
    assert!(s.execute("camera.follow-agents", &json!({"on": false})).is_err());
    assert!(!s.prefs.settings.follow_chosen, "never by an agent");
    s.set_author(crate::agents::Author::Person);
    s.execute("camera.follow-agents", &json!({"on": false, "origin": "button"})).unwrap();
    assert!(s.prefs.settings.follow_chosen, "the person's button");
    // And a migration never writes it, nor touches a chosen value.
    let chosen = crate::settings::Settings { follow_agents: false, follow_chosen: true, settings_version: 8, ..Default::default() }.sanitized();
    assert!(!chosen.follow_agents && chosen.follow_chosen);
    let repaired = crate::settings::Settings { follow_agents: false, settings_version: 8, ..Default::default() }.sanitized();
    assert!(repaired.follow_agents && !repaired.follow_chosen, "repaired, unmarked");
}
