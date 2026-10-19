//! The accent colour, derived for each theme so its contrast holds (Scrim's UI system:
//! `docs/scrim-transfer/ui/README.md` §5, `ui/tokens.json › accentRule`).
//!
//! From one base colour come four tokens per theme:
//! - `mark`: icons, the selected edge, the focus ring and coloured text. It needs 4.5:1 on the
//!   panel, field, raised and floating surfaces, and 3:1 on the canvas and selected surfaces.
//! - `fill`: primary buttons, checked boxes and chips. Some label must reach 4.5:1 on it, and the
//!   fill itself must reach 3:1 against the panel.
//! - `on_fill`: the label on the fill, white or near-black, whichever reads better.
//! - `tint`: a wash of the mark (alpha 0.16 dark, 0.12 light).
//!
//! Only the lightness moves when a colour fails, so the hue stays the person's choice.

use egui::Color32;

/// The near-black label for light fills (the dark theme's panel).
pub const INK: Color32 = Color32::from_rgb(0x11, 0x12, 0x14);
/// Scrim's default accent: orange.
pub const DEFAULT_ACCENT: Color32 = Color32::from_rgb(0xFF, 0x5A, 0x1F);

/// The surfaces a theme's accent must read on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ramp {
    pub canvas: Color32,
    pub panel: Color32,
    pub field: Color32,
    pub raised: Color32,
    pub selected: Color32,
    pub float: Color32,
}

/// The accent tokens for one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accent {
    pub mark: Color32,
    pub fill: Color32,
    pub on_fill: Color32,
    pub tint: Color32,
}

/// The WCAG 2.x relative luminance of an sRGB colour (alpha ignored).
pub fn luminance(c: Color32) -> f64 {
    let ch = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
}

/// The WCAG contrast ratio of two colours, 1 to 21.
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// `#rgb` or `#rrggbb` (the `#` optional), or `None`.
pub fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.trim().trim_start_matches('#');
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    let nib = |i: usize| u8::from_str_radix(h.get(i..i + 1)?, 16).ok().map(|v| v * 17);
    match h.len() {
        3 => Some(Color32::from_rgb(nib(0)?, nib(1)?, nib(2)?)),
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        _ => None,
    }
}

/// `#rrggbb`, lower case.
pub fn hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

/// Hue (degrees), saturation and lightness, each of the last two 0 to 1.
pub fn to_hsl(c: Color32) -> (f64, f64, f64) {
    let (r, g, b) = (f64::from(c.r()) / 255.0, f64::from(c.g()) / 255.0, f64::from(c.b()) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < f64::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

/// The colour for hue `h` (degrees), saturation `s` and lightness `l`, rounded to whole channels.
pub fn from_hsl(h: f64, s: f64, l: f64) -> Color32 {
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    let k = |n: f64| (n + h / 30.0).rem_euclid(12.0);
    let a = s * l.min(1.0 - l);
    let f = |n: f64| l - a * (k(n) - 3.0).min(9.0 - k(n)).clamp(-1.0, 1.0);
    let byte = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(byte(f(0.0)), byte(f(8.0)), byte(f(4.0)))
}

const STEP: f64 = 0.005;
const MAX_STEPS: u32 = 200;

/// Why a base colour can't be an accent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    NotHex,
    Grey,
    NoContrast,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Refusal::NotHex => "Use a hex colour such as #ff5a1f.",
            Refusal::Grey => "This colour is too grey, too light or too dark to read as an accent.",
            Refusal::NoContrast => "No lightness of this colour has enough contrast on this theme.",
        })
    }
}

fn mark_ok(c: Color32, r: &Ramp) -> bool {
    [r.panel, r.field, r.raised, r.float].iter().all(|&s| contrast(c, s) >= 4.5) && [r.canvas, r.selected].iter().all(|&s| contrast(c, s) >= 3.0)
}

fn fill_ok(c: Color32, r: &Ramp) -> bool {
    (contrast(Color32::WHITE, c) >= 4.5 || contrast(INK, c) >= 4.5) && contrast(c, r.panel) >= 3.0
}

/// White or near-black on `fill`, whichever reads better (a tie goes to white).
pub fn on_fill(fill: Color32) -> Color32 {
    if contrast(Color32::WHITE, fill) >= contrast(INK, fill) { Color32::WHITE } else { INK }
}

/// The accent tokens for `base` on a theme with surfaces `ramp` (`dark` decides which way the mark
/// moves: lighter on dark themes, darker on light ones).
pub fn derive(base: Color32, ramp: &Ramp, dark: bool) -> Result<Accent, Refusal> {
    let (h, s, l) = to_hsl(base);
    if s < 0.18 || !(0.03..=0.97).contains(&l) {
        return Err(Refusal::Grey);
    }
    let step = |n: u32, dir: f64| from_hsl(h, s, l + dir * STEP * f64::from(n));
    let mark = if mark_ok(base, ramp) {
        base
    } else {
        let dir = if dark { 1.0 } else { -1.0 };
        (1..=MAX_STEPS).map(|n| step(n, dir)).find(|c| mark_ok(*c, ramp)).ok_or(Refusal::NoContrast)?
    };
    let fill = if fill_ok(base, ramp) {
        base
    } else {
        (1..=MAX_STEPS)
            .map(|n| step(n, 1.0))
            .find(|c| fill_ok(*c, ramp))
            .or_else(|| (1..=MAX_STEPS).map(|n| step(n, -1.0)).find(|c| fill_ok(*c, ramp)))
            .ok_or(Refusal::NoContrast)?
    };
    let alpha = if dark { 0.16 } else { 0.12 };
    let tint = Color32::from_rgba_unmultiplied(mark.r(), mark.g(), mark.b(), (alpha * 255.0_f64).round() as u8);
    Ok(Accent { mark, fill, on_fill: on_fill(fill), tint })
}
