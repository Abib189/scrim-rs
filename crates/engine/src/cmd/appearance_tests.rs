//! Appearance: the accent's presets, custom values and refusals, the theme, and the one default.

use serde_json::json;

use super::{DEFAULT_ACCENT, PRESETS, REQUEST, hex, parse, refusal};
use crate::Session;

#[test]
fn the_default_is_one_constant_and_a_preset() {
    assert_eq!(hex(DEFAULT_ACCENT), "#ff5a1f");
    assert_eq!(PRESETS[0], ("orange", DEFAULT_ACCENT));
    let mut s = Session::new();
    let r = s.execute("pref.appearance", &json!({})).unwrap();
    assert_eq!((&r["accent"], &r["preset"], &r["default"]), (&json!("#ff5a1f"), &json!("orange"), &json!("#ff5a1f")));
    assert_eq!(r["presets"].as_array().map(Vec::len), Some(6));
    assert_eq!(s.prefs.accent, "", "the default is saved as no choice, so a new default reaches everyone");
}

#[test]
fn presets_and_custom_colours_are_kept_and_asked_of_the_ui() {
    let mut s = Session::new();
    s.ui_requests.clear();
    let r = s.execute("pref.accent", &json!({"preset": "Teal"})).unwrap();
    assert_eq!((&r["accent"], &r["preset"]), (&json!("#1fb5a6"), &json!("teal")));
    assert_eq!(s.prefs.accent, "#1fb5a6");
    let [req] = s.ui_requests.as_slice() else { panic!("{:?}", s.ui_requests) };
    assert_eq!((req.kind.as_str(), &req.data), (REQUEST, &r));
    let r = s.execute("pref.accent", &json!({"value": "#3A6"})).unwrap();
    assert_eq!((&r["accent"], &r["preset"]), (&json!("#33aa66"), &json!(null)));
    // No preset and no value: back to the default.
    let r = s.execute("pref.accent", &json!({})).unwrap();
    assert_eq!((&r["accent"], s.prefs.accent.as_str()), (&json!("#ff5a1f"), ""));
    // Saved and read back.
    s.execute("pref.accent", &json!({"value": "#4a86ff"})).unwrap();
    let saved: crate::Prefs = serde_json::from_value(serde_json::to_value(&s.prefs).unwrap()).unwrap();
    assert_eq!(super::accent(&saved), [0x4A, 0x86, 0xFF]);
    let old: crate::Prefs = serde_json::from_value(json!({"theme": "dark"})).unwrap();
    assert_eq!(super::accent(&old), DEFAULT_ACCENT, "preferences from before the accent existed");
}

#[test]
fn colours_that_cant_be_an_accent_are_refused_with_the_reason() {
    let mut s = Session::new();
    s.execute("pref.accent", &json!({"preset": "blue"})).unwrap();
    for (p, why) in [
        (json!({"value": "#808080"}), "too grey"),
        (json!({"value": "#fefefe"}), "too grey"),
        (json!({"value": "#010101"}), "too grey"),
        (json!({"value": "orange"}), "a hex colour"),
        (json!({"value": "#12345"}), "a hex colour"),
        (json!({"preset": "mauve"}), "preset: orange|"),
        (json!({"preset": "blue", "value": "#ff0000"}), "not both"),
    ] {
        let e = s.execute("pref.accent", &p).unwrap_err().to_string();
        assert!(e.contains(why), "{p}: {e}");
    }
    assert_eq!(s.prefs.accent, "#4a86ff", "a refusal changes nothing");
    assert_eq!(parse("ff5a1f"), Some(DEFAULT_ACCENT));
    assert!(refusal(DEFAULT_ACCENT).is_none() && PRESETS.iter().all(|(_, c)| refusal(*c).is_none()));
}

#[test]
fn the_theme_is_a_command_too() {
    let mut s = Session::new();
    assert_eq!(s.execute("pref.theme", &json!({"theme": "Dark"})).unwrap()["theme"], "dark");
    assert_eq!(s.prefs.theme, "dark");
    assert!(s.execute("pref.theme", &json!({"theme": "sepia"})).unwrap_err().to_string().contains("light|dark|system"));
    // The older menu commands set the same preference.
    s.execute("view.themeSystem", &json!({})).unwrap();
    assert_eq!(s.execute("pref.appearance", &json!({})).unwrap()["theme"], "system");
}
