//! Appearance (B4): every preset and a custom accent read at AA in both themes, the default keeps
//! Port 1's tokens exactly, Preferences runs `pref.accent`, and the commands are in Cmd+K.

use egui::Color32;
use egui::accesskit::{Role, Toggled};
use figcraft_engine::Session;
use figcraft_engine::cmd::appearance::PRESETS;
use serde_json::json;

use crate::layers_drag_tests::Ui;
use crate::theme::Tokens;
use crate::theme::accent::contrast;

fn themes() -> [Tokens; 2] {
    [Tokens::dark(), Tokens::light()]
}

#[test]
fn every_accent_reads_at_aa_in_both_themes() {
    let custom = [[0x33, 0xAA, 0x66], [0xFF, 0xD6, 0x00], [0x1A, 0x1A, 0x80], [0xFF, 0x00, 0x00]];
    for c in PRESETS.iter().map(|(_, c)| *c).chain(custom) {
        for base in themes() {
            let t = base.with_accent(Color32::from_rgb(c[0], c[1], c[2]));
            assert!(contrast(t.accent_on, t.accent_fill) >= 4.5, "{c:?}: text on the accent {:.2}:1", contrast(t.accent_on, t.accent_fill));
            for surface in [t.panel, t.field, t.menu] {
                assert!(contrast(t.accent, surface) >= 4.5, "{c:?}: the mark on {surface:?}");
            }
            assert!(contrast(t.accent_fill, t.panel) >= 3.0, "{c:?}: the fill against the panel");
            assert_eq!((t.focus, t.accent_text), (t.accent, t.accent), "{c:?}: one mark for focus and coloured text");
        }
    }
}

#[test]
fn the_default_accent_keeps_the_tokens_as_written() {
    let s = Session::new();
    assert!(Tokens::for_prefs(true, &s.prefs) == Tokens::dark() && Tokens::for_prefs(false, &s.prefs) == Tokens::light());
    assert_eq!(crate::theme::accent::DEFAULT_ACCENT, Color32::from_rgb(0xFF, 0x5A, 0x1F), "read from the engine's one constant");
}

#[test]
fn preferences_and_agents_set_the_accent_with_the_same_command() {
    let mut u = Ui::new(Session::new());
    u.app.ui.preferences = true;
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    let nodes = u.frame_output(vec![]).platform_output.accesskit_update.unwrap().nodes;
    let swatch = |label: &str| nodes.iter().map(|(_, n)| n).find(|n| n.role() == Role::Button && n.label() == Some(label)).cloned();
    assert_eq!(swatch("Orange accent").and_then(|n| n.toggled()), Some(Toggled::True), "the default is selected");
    let teal = swatch("Teal accent").and_then(|n| n.bounds()).unwrap();
    let at = egui::pos2(((teal.x0 + teal.x1) / 2.0) as f32, ((teal.y0 + teal.y1) / 2.0) as f32);
    u.button(at, true);
    u.button(at, false);
    u.frame(vec![]);
    assert_eq!(u.app.session.prefs.accent, "#1fb5a6");
    let want = Tokens::for_prefs(u.app.ui.dark, &u.app.session.prefs);
    assert_eq!(Tokens::get(&u.ctx).accent_fill, want.accent_fill, "the theme takes it on the next frame");
    // An agent's custom colour; a grey one is refused and changes nothing.
    u.app.run("pref.accent", json!({"value": "#33aa66"}));
    assert_eq!(u.app.session.prefs.accent, "#33aa66");
    assert!(u.app.run("pref.accent", json!({"value": "#808080"})).is_none());
    assert_eq!(u.app.session.prefs.accent, "#33aa66");
    u.app.run("pref.accent", json!({}));
    u.frame(vec![]);
    assert_eq!(u.app.session.prefs.accent, "", "Reset is the default again");
}

#[test]
fn the_appearance_commands_are_in_the_actions_palette() {
    let rows: Vec<String> = crate::menus::command_sections("accent").into_iter().flat_map(|(_, rows)| rows.into_iter().map(|r| r.key)).collect();
    assert!(rows.contains(&"command:pref.accent".to_string()), "{rows:?}");
    let rows: Vec<String> = crate::menus::command_sections("theme").into_iter().flat_map(|(_, rows)| rows.into_iter().map(|r| r.key)).collect();
    assert!(rows.contains(&"command:pref.theme".to_string()), "{rows:?}");
}
