//! The accent derivation against Scrim's exported presets (`docs/scrim-transfer/ui/tokens.json ›
//! accentRule`): six base colours, each derived for both themes by Scrim's own code.

use serde_json::Value;

use crate::theme::accent::*;

fn tokens() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/ui/tokens.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn ramp(v: &Value) -> Ramp {
    let c = |k: &str| parse_hex(v[k].as_str().unwrap()).unwrap();
    Ramp { canvas: c("canvas"), panel: c("panel"), field: c("field"), raised: c("raised"), selected: c("selected"), float: c("float") }
}

#[test]
fn every_preset_derives_as_scrim_derived_it_in_both_themes() {
    let t = tokens();
    let rule = &t["accentRule"];
    let presets = rule["presets"].as_object().unwrap();
    let mut checked = 0;
    for (name, p) in presets.iter().filter(|(k, _)| !k.starts_with('$')) {
        let base = parse_hex(p["base"].as_str().unwrap()).unwrap();
        for (theme, dark) in [("dark", true), ("light", false)] {
            let got = derive(base, &ramp(&rule["ramp"][theme]), dark).unwrap();
            let want = &p[theme];
            assert_eq!(hex(got.mark), want["mark"].as_str().unwrap(), "{name} {theme} mark");
            assert_eq!(hex(got.fill), want["fill"].as_str().unwrap(), "{name} {theme} fill");
            assert_eq!(hex(got.on_fill), want["onFill"].as_str().unwrap(), "{name} {theme} on fill");
            checked += 1;
        }
    }
    assert_eq!(checked, 12, "six presets, two themes");
}

#[test]
fn derived_accents_keep_their_contrast_promises() {
    let t = tokens();
    let rule = &t["accentRule"];
    for theme in ["dark", "light"] {
        let r = ramp(&rule["ramp"][theme]);
        let a = derive(DEFAULT_ACCENT, &r, theme == "dark").unwrap();
        for s in [r.panel, r.field, r.raised, r.float] {
            assert!(contrast(a.mark, s) >= 4.5, "{theme}: mark {} on {}", hex(a.mark), hex(s));
        }
        assert!(contrast(a.on_fill, a.fill) >= 4.5, "{theme}: label on the fill");
        assert!(contrast(a.fill, r.panel) >= 3.0, "{theme}: the fill against the panel");
    }
}

#[test]
fn greys_and_non_hex_are_refused_with_a_reason() {
    let r = ramp(&tokens()["accentRule"]["ramp"]["dark"]);
    assert_eq!(derive(parse_hex("#808080").unwrap(), &r, true), Err(Refusal::Grey));
    assert_eq!(parse_hex("orange"), None);
    assert_eq!(parse_hex("#12345"), None);
    assert_eq!(parse_hex("#f50"), Some(egui::Color32::from_rgb(0xFF, 0x55, 0x00)));
    assert_eq!(Refusal::NotHex.to_string(), "Use a hex colour such as #ff5a1f.");
}

#[test]
fn contrast_is_wcag_2() {
    assert!((contrast(egui::Color32::WHITE, egui::Color32::BLACK) - 21.0).abs() < 1e-9);
    assert!((contrast(egui::Color32::WHITE, egui::Color32::WHITE) - 1.0).abs() < 1e-9);
    // #767676 on white is the classic 4.54:1.
    assert!((contrast(parse_hex("#767676").unwrap(), egui::Color32::WHITE) - 4.54).abs() < 0.01);
}
