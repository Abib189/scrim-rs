//! Stage K2: while a modal (Settings) is open, the wheel, trackpad scroll and pinch anywhere over the
//! window reach nothing behind it: not the canvas's camera, not follow (which would pause), not a
//! panel's scroll. Over a dialog that isn't modal (Missing fonts, Local variables, Export, a Connect
//! confirmation), they don't reach what's under the dialog either. Closed, they work again.

use egui::{Event, Modifiers, MouseWheelUnit, Pos2, TouchPhase, pos2, vec2};
use serde_json::json;

use crate::live_tests::{Harness, text_center};

fn view(h: &Harness) -> (f64, f64, f64) {
    let v = &h.app.session.ds().unwrap().view;
    (v.cx, v.cy, v.zoom)
}

/// What the wheel and a pinch could change behind a dialog.
fn behind(h: &mut Harness) -> ((f64, f64, f64), &'static str, String) {
    let ctx = h.ctx.clone();
    let layers = crate::control::handle(&mut h.app, &ctx, "ui.inspect", &json!({})).unwrap()["layers"].to_string();
    (view(h), figcraft_engine::cmd::camera::follow_state(&h.app.session), layers)
}

/// A wheel turn, a trackpad scroll and a pinch at `at`.
fn scroll_and_pinch(h: &mut Harness, at: Pos2) {
    h.frame(vec![Event::PointerMoved(at)]);
    for e in [
        Event::MouseWheel { unit: MouseWheelUnit::Line, delta: vec2(0.0, -3.0), phase: TouchPhase::Move, modifiers: Modifiers::NONE },
        Event::MouseWheel { unit: MouseWheelUnit::Point, delta: vec2(-40.0, -60.0), phase: TouchPhase::Move, modifiers: Modifiers::NONE },
        Event::Zoom(1.25),
    ] {
        h.frame(vec![e]);
    }
    h.frames(3);
}

/// An agent's edit, so following has work (a person's scroll on the canvas would pause it).
fn agent_work(h: &mut Harness) {
    let ctx = h.ctx.clone();
    crate::control::handle_route(
        &mut h.app,
        &ctx,
        "mcp (http)",
        "engine.execute",
        &json!({"command": "frame.create", "params": {"x": 0, "y": 0, "width": 200, "height": 200, "name": "Work"}}),
    )
    .unwrap();
    h.frames(40);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&h.app.session), "following");
}

#[test]
fn with_settings_open_nothing_behind_it_scrolls_zooms_or_pauses() {
    let (mut h, _) = Harness::new();
    agent_work(&mut h);
    h.app.run("settings.open", json!({}));
    h.app.handle_ui_requests();
    let shapes = h.frames(3);
    assert!(h.app.ui.preferences);
    let title = text_center(&shapes, "Settings");
    let before = behind(&mut h);
    // Over the dialog, over the canvas beside it, over the Layers panel and the right panel.
    for at in [title + vec2(0.0, 200.0), pos2(1180.0, 450.0), pos2(150.0, 450.0), pos2(1320.0, 450.0)] {
        scroll_and_pinch(&mut h, at);
        assert_eq!(behind(&mut h), before, "nothing behind Settings moved for the wheel and pinch at {at:?}");
    }
    // Closed: they reach the canvas again.
    h.app.ui.preferences = false;
    h.frames(3);
    scroll_and_pinch(&mut h, pos2(700.0, 450.0));
    assert_ne!(view(&h), before.0, "the camera moves again");
}

/// Follow's own check (the person scrolled the canvas: pause) on its harness, where the canvas is
/// sized for following: under Settings it doesn't fire; closed, it does.
#[test]
fn under_settings_a_scroll_on_the_canvas_doesnt_pause_following() {
    use crate::follow_agent::tests::{frames, poster, scene};
    let (mut u, _) = scene();
    poster(&mut u, 300.0);
    frames(&mut u, 40);
    let wheel = |u: &mut crate::layers_drag_tests::Ui| {
        let at = u.app.canvas.rect.center() + vec2(-300.0, 0.0);
        u.frame(vec![Event::PointerMoved(at)]);
        u.frame(vec![Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -120.0),
            phase: TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }]);
        u.frame(vec![Event::Zoom(1.25)]);
        frames(u, 30);
    };
    u.app.ui.preferences = true;
    frames(&mut u, 3);
    let v = u.app.session.ds().unwrap().view.clone();
    wheel(&mut u);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following", "under Settings: still following");
    let w = &u.app.session.ds().unwrap().view;
    assert_eq!((w.cx, w.cy, w.zoom), (v.cx, v.cy, v.zoom), "and the camera unmoved");
    u.app.ui.preferences = false;
    frames(&mut u, 3);
    wheel(&mut u);
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "paused", "closed: the person's scroll pauses it again");
}

#[test]
fn over_a_dialog_the_wheel_and_pinch_dont_reach_the_canvas() {
    type Open = fn(&mut Harness);
    let dialogs: [(&str, Open, &str); 4] = [
        ("Missing fonts", |h| drop(h.app.run("text.missingFonts", json!({}))), "Missing fonts"),
        ("Local variables", |h| h.app.ui.variables = Some(Default::default()), "Local variables"),
        ("Export", |h| h.app.ui.export_preview = true, "Export"),
        (
            "a Connect confirmation",
            |h| {
                h.app.session.agents.mcp.port = Some(7601);
                crate::connect_flow::request(&mut h.app, figcraft_engine::cmd::mcp::RUN_DIALOG);
            },
            "Run this command?",
        ),
    ];
    for (name, open, title) in dialogs {
        let (mut h, _) = Harness::new();
        agent_work(&mut h);
        open(&mut h);
        h.app.handle_ui_requests();
        let shapes = h.frames(4);
        let Some(at) = crate::live_tests::texts(&shapes).into_iter().filter(|(t, _, _)| t == title).map(|(_, p, s)| p + s / 2.0).next_back() else {
            panic!("{name}: its window, titled {title:?}, isn't shown");
        };
        let before = behind(&mut h);
        scroll_and_pinch(&mut h, at + vec2(0.0, 40.0));
        assert_eq!(behind(&mut h), before, "{name}: the wheel and pinch over it reached what's behind");
    }
}
