//! Appearance (`docs/design/panel-redesign.md`, B4): the theme and the accent colour, as commands
//! so agents set them as the person does in Preferences. The accent's default is
//! [`DEFAULT_ACCENT`], the one place it is written; the UI derives each theme's accent tokens from
//! the chosen base (`ui-egui`'s `accent::derive`, Port 1), with text on the accent picked for AA
//! (4.5:1). What doesn't depend on a theme is checked here: a hex colour, not too grey, light or
//! dark (the derivation's own first rule).

use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::*;
use crate::{Result, Session, bad, spec};

/// Scrim's accent: orange `#ff5a1f`.
pub const DEFAULT_ACCENT: [u8; 3] = [0xFF, 0x5A, 0x1F];

/// The presets Preferences offers (Scrim's six, `docs/scrim-transfer/ui/tokens.json › accentRule`).
pub const PRESETS: [(&str, [u8; 3]); 6] = [
    ("orange", DEFAULT_ACCENT),
    ("blue", [0x4A, 0x86, 0xFF]),
    ("violet", [0x8E, 0x7B, 0xFF]),
    ("green", [0x2F, 0xB3, 0x6B]),
    ("pink", [0xEC, 0x4F, 0x98]),
    ("teal", [0x1F, 0xB5, 0xA6]),
];

pub fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// `#rgb` or `#rrggbb` (the `#` optional).
pub fn parse(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    if !h.is_ascii() {
        return None;
    }
    let digit = |i: usize, n: usize| u8::from_str_radix(h.get(i..i + n)?, 16).ok();
    match h.len() {
        3 => Some([digit(0, 1)? * 17, digit(1, 1)? * 17, digit(2, 1)? * 17]),
        6 => Some([digit(0, 2)?, digit(2, 2)?, digit(4, 2)?]),
        _ => None,
    }
}

/// Why `c` can't be an accent on any theme: too grey, too light or too dark (HSL saturation under
/// 0.18, or lightness outside 0.03–0.97), as the derivation refuses it.
pub fn refusal(c: [u8; 3]) -> Option<&'static str> {
    let [r, g, b] = c.map(|v| f64::from(v) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
    (s < 0.18 || !(0.03..=0.97).contains(&l)).then_some(GREY)
}

/// The refusal's words (the UI shows them too).
pub const GREY: &str = "this colour is too grey, too light or too dark to read as an accent";

/// The accent the preferences ask for: their colour, or the default (empty or unreadable).
pub fn accent(prefs: &crate::Prefs) -> [u8; 3] {
    parse(&prefs.accent).filter(|c| refusal(*c).is_none()).unwrap_or(DEFAULT_ACCENT)
}

/// The UI request that applies the theme and accent again (`{theme, accent}`).
pub const REQUEST: &str = "appearance";

fn state(s: &Session) -> Value {
    let c = accent(&s.prefs);
    let preset = PRESETS.iter().find(|(_, p)| *p == c).map(|(name, _)| *name);
    json!({
        "theme": s.prefs.theme,
        "accent": hex(c),
        "preset": preset,
        "default": hex(DEFAULT_ACCENT),
        "presets": PRESETS.iter().map(|(name, c)| json!({"name": name, "color": hex(*c)})).collect::<Vec<_>>(),
    })
}

fn set_accent(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "pref.accent";
    let color = match (string(p, "preset"), string(p, "value")) {
        (Some(_), Some(_)) => return Err(bad(cmd, "give `preset` or `value`, not both")),
        (Some(name), None) => {
            let name = name.to_ascii_lowercase();
            PRESETS.iter().find(|(n, _)| *n == name).map(|(_, c)| *c).ok_or_else(|| bad(cmd, "preset: orange|blue|violet|green|pink|teal"))?
        }
        (None, Some(v)) => {
            let c = parse(&v).ok_or_else(|| bad(cmd, "value: a hex colour such as #ff5a1f"))?;
            if let Some(why) = refusal(c) {
                return Err(bad(cmd, why));
            }
            c
        }
        (None, None) => DEFAULT_ACCENT,
    };
    s.prefs.accent = if color == DEFAULT_ACCENT { String::new() } else { hex(color) };
    let r = state(s);
    s.request_ui(REQUEST, r.clone());
    Ok(r)
}

fn set_theme(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "pref.theme";
    let theme = string(p, "theme").map(|t| t.to_ascii_lowercase()).ok_or_else(|| bad(cmd, "`theme` is required: light|dark|system"))?;
    if !matches!(theme.as_str(), "light" | "dark" | "system") {
        return Err(bad(cmd, "theme: light|dark|system"));
    }
    s.prefs.theme = theme;
    let r = state(s);
    s.request_ui(REQUEST, r.clone());
    Ok(r)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "pref.accent",
            "Accent colour",
            "{preset?: orange|blue|violet|green|pink|teal, value?: #rrggbb (neither: the default, #ff5a1f)} → {theme, accent, preset, default, presets}  (text on the accent is chosen for 4.5:1; a colour too grey, light or dark is refused)",
            false,
            set_accent
        ),
        spec!("pref.theme", "Theme", "{theme: light|dark|system} → {theme, accent, preset, default, presets}", false, set_theme),
        spec!("pref.appearance", "Appearance", "{} → {theme, accent, preset, default, presets}", false, |s, _| Ok(state(s))),
    ]
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
