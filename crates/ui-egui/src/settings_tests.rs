//! Settings (C3a): where it opens from, Done and Escape, an agent's `settings.open`, one theme
//! setting behind both Settings and the tab bar's sun/moon, animations, keeping unsaved work, and
//! the home sidebar's pinned Settings row.

use egui::accesskit::{Node, Role};
use egui::{Event, Key};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::live_tests::{Harness, has_text, text_center};

fn key(h: &mut Harness, key: Key, command: bool) {
    let modifiers = if command { egui::Modifiers::COMMAND } else { egui::Modifiers::NONE };
    for pressed in [true, false] {
        h.frame(vec![Event::Key { key, physical_key: None, pressed, repeat: false, modifiers }]);
    }
}

#[test]
fn settings_opens_from_its_command_cmd_comma_and_closes_with_done_or_escape() {
    let (mut h, _) = Harness::new();
    h.frames(2);
    h.app.run("settings.open", json!({}));
    let shapes = h.frames(2);
    assert!(h.app.ui.preferences && h.app.ui.settings_section == "general");
    // (The pane scrolls: the sections further down aren't drawn until scrolled to.)
    for t in ["Settings", "General", "Appearance", "Theme", "Animations", "Agents at work", "Done"] {
        assert!(has_text(&shapes, t), "Settings shows {t:?}");
    }
    h.click(text_center(&shapes, "Done"));
    h.frames(2);
    assert!(!h.app.ui.preferences, "Done closes it");
    key(&mut h, Key::Comma, true);
    h.frames(2);
    assert!(h.app.ui.preferences, "Cmd+, opens it (app.preferences)");
    key(&mut h, Key::Escape, false);
    h.frames(1);
    assert!(!h.app.ui.preferences, "Escape closes it");
}

#[test]
fn an_agents_settings_open_returns_the_settings_and_opens_nothing() {
    let (mut h, _) = Harness::new();
    let ctx = h.ctx.clone();
    let r =
        crate::control::handle(&mut h.app, &ctx, "engine.execute", &json!({"command": "settings.open", "params": {"section": "general"}})).unwrap();
    assert_eq!(r["section"], "general");
    assert_eq!(r["motion"], "system");
    assert!(r["sections"].as_array().is_some_and(|s| s.len() == 7));
    h.frames(2);
    assert!(!h.app.ui.preferences, "programmatic calls open no dialogs");
    let e = h.app.session.execute("settings.open", &json!({"section": "usage"})).unwrap_err().to_string();
    assert!(e.contains("section:"), "{e}");
}

/// Addendum 1: Settings ▸ General ▸ Theme and the tab bar's sun/moon are one setting.
#[test]
fn the_theme_in_settings_and_the_tab_bars_toggle_follow_each_other() {
    let (mut h, _) = Harness::new();
    h.app.run("pref.theme", json!({"theme": "light"}));
    h.frames(3);
    assert!(!h.app.ui.dark);
    // The tab bar's toggle (its AccessKit button) turns it dark …
    h.ctx.enable_accesskit();
    h.frames(1);
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0))), ..Default::default() };
    let app = &mut h.app;
    let mut out = h.ctx.run_ui(input, |ui| app.ui(ui));
    out.textures_delta.clear();
    let toggle = out
        .platform_output
        .accesskit_update
        .unwrap()
        .nodes
        .into_iter()
        .map(|(_, n)| n)
        .find(|n| n.label() == Some("Toggle light/dark theme"))
        .unwrap();
    let b = toggle.bounds().unwrap();
    h.click(egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32));
    h.frames(3);
    assert_eq!(h.app.session.prefs.theme, "dark");
    assert!(h.app.ui.dark);
    // … and Settings shows Dark; choosing Light there turns the tab bar's icon back.
    h.app.run("settings.open", json!({}));
    let shapes = h.frames(2);
    h.click(text_center(&shapes, "Light"));
    h.frames(3);
    assert_eq!(h.app.session.prefs.theme, "light");
    assert!(!h.app.ui.dark, "the tab bar follows Settings");
}

#[test]
fn animations_follow_system_on_or_off() {
    let mut u = Ui::new(Session::new());
    for (m, reduce) in [("off", true), ("on", false), ("system", false)] {
        u.app.run("pref.motion", json!({"motion": m}));
        assert_eq!(u.app.reduce_motion(), reduce, "{m}");
    }
    // The old switch is the same setting.
    u.app.run("pref.reduceMotion", json!({"on": true}));
    assert_eq!(u.app.session.prefs.settings.motion, "off");
    assert!(u.app.run("pref.motion", json!({"motion": "sometimes"})).is_none());
}

#[test]
fn keeping_unsaved_work_is_a_kept_setting() {
    let mut s = Session::new();
    assert!(s.prefs.settings.preserve_unsaved, "on by default");
    assert_eq!(s.execute("pref.preserveUnsaved", &json!({})).unwrap(), json!({"on": false}));
    let back: figcraft_engine::Prefs = serde_json::from_value(serde_json::to_value(&s.prefs).unwrap()).unwrap();
    assert!(!back.settings.preserve_unsaved);
    let missing: figcraft_engine::Prefs = serde_json::from_value(json!({"theme": "dark"})).unwrap();
    assert!(missing.settings.preserve_unsaved && missing.settings.motion == "system", "missing keys fall back");
    let odd: figcraft_engine::Prefs = serde_json::from_value(json!({"motion": "maybe"})).unwrap();
    assert_eq!(odd.settings.sanitized().motion, "system", "an invalid value falls back");
}

fn tree(u: &mut Ui) -> Vec<Node> {
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    u.frame_output(vec![]).platform_output.accesskit_update.unwrap().nodes.into_iter().map(|(_, n)| n).collect()
}

#[test]
fn the_home_sidebar_has_settings_pinned_at_the_bottom() {
    let mut u = Ui::new(Session::new());
    u.app.ui.home = true;
    let nodes = tree(&mut u);
    let settings = nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some("Settings")).unwrap();
    let templates = nodes.iter().find(|n| n.role() == Role::Tab && n.label() == Some("Templates")).unwrap();
    let (s, t) = (settings.bounds().unwrap(), templates.bounds().unwrap());
    assert!((s.y1 - 892.0).abs() < 1.0, "pinned 8 px above the bottom: {s:?}");
    assert!((s.y1 - s.y0 - 32.0).abs() < 0.5 && (s.x0 - t.x0).abs() < 0.5 && (s.x1 - t.x1).abs() < 0.5, "same row size: {s:?} {t:?}");
    // The rows above keep their places (Recents, Online, Templates on a 34 px step).
    let rows: Vec<f64> = ["Recents", "Online", "Templates"]
        .iter()
        .map(|l| nodes.iter().find(|n| n.role() == Role::Tab && n.label() == Some(*l)).unwrap().bounds().unwrap().y0)
        .collect();
    assert_eq!(rows[1] - rows[0], 34.0);
    assert_eq!(rows[2] - rows[1], 34.0);
    u.app.run("settings.open", json!({}));
    let nodes = tree(&mut u);
    assert!(
        nodes.iter().any(|n| n.role() == Role::Tab && n.label() == Some("General") && n.is_selected() == Some(true)),
        "the rail's current section"
    );
}
