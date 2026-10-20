//! Design tokens and fonts: Scrim's UI system (`docs/scrim-transfer/ui/`), dark and light, one
//! accent derived per theme for contrast ([`accent`]).

use std::sync::Arc;

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle, Visuals};

#[path = "accent.rs"]
pub mod accent;
#[cfg(test)]
#[path = "accent_tests.rs"]
mod accent_tests;
#[cfg(test)]
#[path = "theme_tokens_tests.rs"]
mod theme_tokens_tests;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub canvas: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub icon: Color32,
    pub icon_secondary: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub field: Color32,
    pub field_hover_border: Color32,
    /// The accent's mark: icons, the selected edge, the focus ring, coloured text. 4.5:1 on the
    /// panels and fields.
    pub accent: Color32,
    /// The accent's fill (primary buttons, checked boxes, chips) and its hover.
    pub accent_fill: Color32,
    pub accent_hover: Color32,
    /// The accent fill while pressed.
    pub accent_pressed: Color32,
    /// The accent the person chose (the canvas overlays derive theirs from it per page).
    pub accent_base: Color32,
    /// The label on the accent fill: 4.5:1.
    pub accent_on: Color32,
    /// A wash of the mark, for drop targets and highlights.
    pub accent_tint: Color32,
    pub accent_text: Color32,
    pub selected_row: Color32,
    pub selected_row_child: Color32,
    pub component: Color32,
    pub component_row: Color32,
    pub danger: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub guide: Color32,
    pub tab_bar: Color32,
    pub tab_active: Color32,
    pub shadow: Color32,
    pub menu: Color32,
    pub menu_text: Color32,
    pub menu_hover: Color32,
    pub menu_dim: Color32,
    /// The ring around the control with the keyboard (keyboard navigation): at least 3:1 against
    /// the panels and the canvas.
    pub focus: Color32,
}

/// Scrim's default accent fill (orange), the same in both themes; its label is [`ACCENT_ON`]. Where
/// the tokens are at hand, use `t.accent_fill` / `t.accent_on` (and `t.accent` for marks).
pub const ACCENT: Color32 = accent::DEFAULT_ACCENT;
/// Scrim's brand orange: the logo "S", file badges, the About tile and the app icon keep it whatever
/// the accent (`docs/design/accent-sweep.md`); its label is [`BRAND_ON`]. Everything interactive
/// reads the accent instead ([`ink`]).
pub const BRAND: Color32 = accent::DEFAULT_ACCENT;
pub const BRAND_ON: Color32 = accent::INK;
/// The live accent where code paints it (Stage G).
pub use crate::accent_ink as ink;
/// The label on [`ACCENT`]: near-black, 4.5:1 or better (white on this orange is about 3:1).
pub const ACCENT_ON: Color32 = accent::INK;
pub const COMPONENT: Color32 = Color32::from_rgb(0x97, 0x47, 0xFF);
/// Handoff status badges (white text on these, in both themes).
pub const DEV_READY: Color32 = Color32::from_rgb(0x14, 0xAE, 0x5C);
pub const DEV_CHANGED: Color32 = Color32::from_rgb(0xD9, 0x7A, 0x06);
pub const DEV_COMPLETED: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x73);

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

impl Tokens {
    /// Scrim's light ramp (`ui/tokens.json › color.light`).
    pub fn light() -> Self {
        Self {
            dark: false,
            bg: hex(0xFFFFFF),
            panel: hex(0xFFFFFF),
            canvas: hex(0xECECEA),
            border: hex(0xE8E8E6),
            border_strong: hex(0xD4D4D1),
            text: hex(0x18191B),
            text_secondary: hex(0x44474C),
            text_tertiary: hex(0x5F6268),
            icon: hex(0x44474C),
            icon_secondary: hex(0x5F6268),
            hover: hex(0xF4F4F3),
            pressed: hex(0xEBEBEA),
            field: hex(0xF4F4F3),
            field_hover_border: hex(0xD4D4D1),
            // The accent derived for this ramp (`accent::derive`; checked by theme_tokens_tests).
            accent: hex(0xC53400),
            accent_fill: ACCENT,
            accent_hover: hex(0xEB531D),
            accent_pressed: hex(0xF54100),
            accent_base: ACCENT,
            accent_on: ACCENT_ON,
            accent_tint: Color32::from_rgba_unmultiplied(0xC5, 0x34, 0x00, 31),
            accent_text: hex(0xC53400),
            selected_row: hex(0xE1E1DF),
            selected_row_child: hex(0xF4F4F3),
            component: COMPONENT,
            component_row: hex(0xF1E9FF),
            danger: hex(0xCF2A2A),
            // C1.5: 5.4:1 on the panel as text, 3:1 or more as a mark everywhere (the old #14AE5C was 2.9:1).
            success: hex(0x0B7A3E),
            warning: hex(0x985F08),
            guide: hex(0xD61F69),
            tab_bar: hex(0xECECEA),
            tab_active: hex(0xFFFFFF),
            shadow: Color32::from_black_alpha(0x1A),
            menu: hex(0xFFFFFF),
            menu_text: hex(0x18191B),
            menu_hover: hex(0xEBEBEA),
            menu_dim: hex(0x5F6268),
            focus: hex(0xC53400),
        }
    }

    /// Scrim's dark ramp (`ui/tokens.json › color.dark`).
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: hex(0x111214),
            panel: hex(0x111214),
            canvas: hex(0x17181A),
            border: hex(0x232428),
            border_strong: hex(0x313338),
            text: hex(0xECECEE),
            text_secondary: hex(0xA7AAB0),
            text_tertiary: hex(0x91959C),
            icon: hex(0xA7AAB0),
            icon_secondary: hex(0x91959C),
            hover: hex(0x1B1C1F),
            pressed: hex(0x232428),
            field: hex(0x1B1C1F),
            field_hover_border: hex(0x313338),
            accent: ACCENT,
            accent_fill: ACCENT,
            accent_hover: hex(0xEB531D),
            accent_pressed: hex(0xF54100),
            accent_base: ACCENT,
            accent_on: ACCENT_ON,
            accent_tint: Color32::from_rgba_unmultiplied(0xFF, 0x5A, 0x1F, 41),
            accent_text: ACCENT,
            selected_row: hex(0x2C2D32),
            selected_row_child: hex(0x1B1C1F),
            component: hex(0xD27CFF),
            component_row: hex(0x3E3052),
            danger: hex(0xFF5C5C),
            success: hex(0x14AE5C),
            warning: hex(0xE6A23A),
            guide: hex(0xFF4D94),
            tab_bar: hex(0x17181A),
            tab_active: hex(0x111214),
            shadow: Color32::from_black_alpha(0x66),
            menu: hex(0x232428),
            menu_text: hex(0xECECEE),
            menu_hover: hex(0x2C2D32),
            menu_dim: hex(0x91959C),
            focus: ACCENT,
        }
    }

    /// The theme for `dark`, with the accent the preferences ask for (`pref.accent`).
    pub fn for_prefs(dark: bool, prefs: &figcraft_engine::Prefs) -> Self {
        let [r, g, b] = figcraft_engine::cmd::appearance::accent(prefs);
        (if dark { Self::dark() } else { Self::light() }).with_accent(Color32::from_rgb(r, g, b))
    }

    /// These tokens with the accent derived from `base` for this theme (Port 1's rule: the mark
    /// reads at 4.5:1 on the panels, and the label on the fill, white or near-black, at 4.5:1). The
    /// default accent keeps the tokens as written; a base the rule refuses keeps them too.
    pub fn with_accent(mut self, base: Color32) -> Self {
        if base == ACCENT {
            return self;
        }
        let ramp = accent::Ramp {
            canvas: self.canvas,
            panel: self.panel,
            field: self.field,
            raised: self.pressed,
            selected: self.selected_row,
            float: self.menu,
        };
        let Ok(a) = accent::derive(base, &ramp, self.dark) else { return self };
        let (h, s, l) = accent::to_hsl(a.fill);
        self.accent = a.mark;
        self.accent_text = a.mark;
        self.focus = a.mark;
        self.accent_fill = a.fill;
        self.accent_hover = accent::from_hsl(h, s, (l - 0.04).max(0.0));
        self.accent_pressed = accent::from_hsl(h, s, (l - 0.08).max(0.0));
        self.accent_base = base;
        self.accent_on = a.on_fill;
        self.accent_tint = a.tint;
        self
    }

    pub fn get(ctx: &Context) -> Tokens {
        ctx.data(|d| d.get_temp::<Tokens>(tokens_id())).unwrap_or_else(Tokens::light)
    }
}

fn tokens_id() -> egui::Id {
    egui::Id::new("figcraft_tokens")
}

/// Scrim's one motion duration (120 ms; `ui/tokens.json › motion`).
const ANIMATION_TIME: f32 = 0.12;

/// Install tokens and egui visuals.
pub fn apply(ctx: &Context, t: Tokens) {
    apply_with(ctx, t, false);
}

/// [`apply`], with egui's own animations (hover fades, openings, smooth scrolling to a target) off
/// when the system or the preferences ask for reduced motion. Prototype transitions still play.
pub fn apply_with(ctx: &Context, t: Tokens, reduce_motion: bool) {
    ctx.data_mut(|d| d.insert_temp(tokens_id(), t));
    crate::accent_ink::install(&t);
    ctx.set_visuals(visuals(t));
    ctx.global_style_mut(|s| {
        // Scrim's rhythm: 24 px controls and a 4 px gap make 28 px rows (`ui/README.md` §6).
        s.spacing.item_spacing = egui::vec2(8.0, 4.0);
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.spacing.interact_size = egui::vec2(24.0, 24.0);
        s.spacing.menu_margin = egui::Margin::same(4);
        s.spacing.window_margin = egui::Margin::same(12);
        s.text_styles = [
            // Scrim's type scale: small 11, body 12, title 13 (`ui/tokens.json › typography`).
            (TextStyle::Small, FontId::new(11.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Heading, FontId::new(13.0, FontFamily::Name("semibold".into()))),
            (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
        ]
        .into();
        s.interaction.tooltip_delay = 0.4;
        s.animation_time = if reduce_motion { 0.0 } else { ANIMATION_TIME };
        s.scroll_animation = if reduce_motion { egui::style::ScrollAnimation::none() } else { egui::style::ScrollAnimation::default() };
    });
}

/// egui visuals for tokens `t`.
fn visuals(t: Tokens) -> Visuals {
    let mut v = if t.dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = t.panel;
    // Menus, popovers, dialogs and tooltips share the floating surface (`ui/README.md` §7).
    v.window_fill = t.menu;
    v.extreme_bg_color = t.field;
    v.faint_bg_color = t.hover;
    v.override_text_color = Some(t.text);
    v.window_stroke = Stroke::new(1.0, t.border_strong);
    v.window_corner_radius = egui::CornerRadius::same(8);
    v.menu_corner_radius = egui::CornerRadius::same(8);
    // Selection is neutral; the accent is only an edge or a mark (`ui/README.md` §4).
    v.selection.bg_fill = t.selected_row;
    v.selection.stroke = Stroke::new(1.0, t.text);
    v.hyperlink_color = t.text_secondary;
    v.warn_fg_color = t.warning;
    v.error_fg_color = t.danger;
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open, &mut v.widgets.noninteractive] {
        w.corner_radius = egui::CornerRadius::same(4);
        w.expansion = 0.0;
        w.fg_stroke = Stroke::new(1.0, t.text);
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.bg_fill = t.panel;
    v.widgets.inactive.bg_fill = t.field;
    v.widgets.inactive.weak_bg_fill = t.field;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.bg_fill = t.pressed;
    v.widgets.hovered.weak_bg_fill = t.pressed;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.field_hover_border);
    v.widgets.active.bg_fill = t.pressed;
    v.widgets.active.weak_bg_fill = t.selected_row;
    v.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);
    v.popup_shadow = egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: t.shadow };
    v.window_shadow = egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: t.shadow };
    v.text_cursor.stroke = Stroke::new(1.5, t.text);
    v
}

/// Draw `f` with the dark tokens and visuals: controls on the dark popovers (the style browser's
/// style editor), whichever theme the app shows.
pub fn on_dark<R>(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let ctx = ui.ctx().clone();
    let saved = Tokens::get(&ctx);
    let dark = Tokens::dark();
    ctx.data_mut(|d| d.insert_temp(tokens_id(), dark));
    let r = ui
        .scope(|ui| {
            *ui.visuals_mut() = visuals(dark);
            f(ui)
        })
        .inner;
    ctx.data_mut(|d| d.insert_temp(tokens_id(), saved));
    r
}

/// UI fonts: Inter (from the craft-fonts build input) when present, else egui's defaults; with
/// `system`, also system faces for scripts those lack (Hebrew, joined Arabic) so names in them show,
/// and faces for the CJK scripts in `cjk` ([`crate::cjk`]).
pub fn install_fonts(ctx: &Context, system: bool, cjk: u8) {
    let mut fonts = FontDefinitions::default();
    let find = |family: &str, style: &str| figcraft_text::CRAFT_FONTS.iter().find(|f| f.family == family && f.style == style).map(|f| f.bytes);
    let regular = find("Inter", "Regular");
    let semibold = find("Inter", "SemiBold").or_else(|| find("Inter", "Bold"));
    if let Some(r) = regular {
        fonts.font_data.insert("inter".into(), Arc::new(FontData::from_static(r)));
        if let Some(list) = fonts.families.get_mut(&FontFamily::Proportional) {
            list.insert(0, "inter".into());
        }
    }
    let sb_name = if let Some(sb) = semibold {
        fonts.font_data.insert("inter-semibold".into(), Arc::new(FontData::from_static(sb)));
        "inter-semibold"
    } else {
        "inter"
    };
    let mut semi: Vec<String> = Vec::new();
    if fonts.font_data.contains_key(sb_name) {
        semi.push(sb_name.into());
    }
    semi.extend(fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
    fonts.families.insert(FontFamily::Name("semibold".into()), semi);
    // CJK and other scripts from craft-fonts as fallbacks (Noto Sans CJK only with FIGCRAFT_EMBED_CJK).
    const BUILT_IN: [&str; 4] = ["Inter", "BIZ UDPGothic", "Noto Sans CJK SC", "Noto Sans Arabic"];
    let lang = crate::i18n::current().map_or("en", |l| l.code);
    // A Chinese UI draws Han in a Chinese face: the built-in Japanese one goes after the system faces.
    let late = |family: &str| family == "BIZ UDPGothic" && crate::cjk::language(lang) == crate::cjk::HAN;
    let craft = |last: bool| {
        figcraft_text::CRAFT_FONTS
            .iter()
            .filter(move |f| f.family != "Inter" && f.style == "Regular" && BUILT_IN.contains(&f.family) && late(f.family) == last)
            .map(|f| (format!("craft-{}", f.family), FontData::from_static(f.bytes)))
    };
    let mut extra: Vec<(String, FontData)> = craft(false).collect();
    if system {
        // What the built-in faces above already draw.
        let embedded = |c: char| {
            BUILT_IN
                .into_iter()
                .filter(|f| !late(f) && figcraft_text::CRAFT_FONTS.iter().any(|cf| cf.family == *f))
                .filter_map(|f| figcraft_text::find_face(f, "Regular"))
                .any(|x| x.has_char(c))
        };
        let mut taken = Vec::new();
        extra.extend(crate::bidi::script_fallbacks(&embedded, &mut taken));
        extra.extend(key_symbol_fallbacks(&embedded, &mut taken));
        extra.extend(crate::cjk::faces(cjk, lang, &embedded).iter().map(|f| (format!("system-{}-{}", f.family, f.style), font_data(f))));
    }
    extra.extend(craft(true));
    for (key, data) in extra {
        fonts.font_data.insert(key.clone(), Arc::new(data));
        for fam in [FontFamily::Proportional, FontFamily::Name("semibold".into())] {
            if let Some(list) = fonts.families.get_mut(&fam) {
                list.push(key.clone());
            }
        }
    }
    ctx.set_fonts(fonts);
}

/// The glyphs macOS shortcuts are written with ([`figcraft_engine::keymap::display`]).
const KEY_SYMBOLS: &str = "⌃⌥⇧⌘⌫↵⇥⇞⇟←→↑↓";
/// System families that have them, in order of preference.
const KEY_FAMILIES: [&str; 6] = ["Lucida Grande", "Apple Symbols", "Segoe UI Symbol", "DejaVu Sans", "Noto Sans Symbols 2", "FreeSans"];

/// System faces for the shortcut glyphs the UI's own fonts lack (egui's default face has ⌘ but
/// not ⌥ or ↵); `taken` as for [`system_fallbacks`].
fn key_symbol_fallbacks(have: &dyn Fn(char) -> bool, taken: &mut Vec<Arc<figcraft_text::Face>>) -> Vec<(String, FontData)> {
    system_fallbacks(KEY_SYMBOLS.chars(), &KEY_FAMILIES, have, taken)
}

/// System faces, the first of `families` that has each, for the `samples` neither the UI's own
/// fonts (`have`) nor the faces already `taken` draw. Each face found joins `taken`, so a face that
/// several lists name is added once.
pub(crate) fn system_fallbacks(
    samples: impl IntoIterator<Item = char>,
    families: &[&str],
    have: &dyn Fn(char) -> bool,
    taken: &mut Vec<Arc<figcraft_text::Face>>,
) -> Vec<(String, FontData)> {
    let mut out = Vec::new();
    for c in samples {
        if have(c) || taken.iter().any(|f| f.has_char(c)) {
            continue;
        }
        let Some(face) = figcraft_text::face_with_char(c, families) else { continue };
        out.push((format!("system-{}-{}", face.family, face.style), font_data(&face)));
        taken.push(face);
    }
    out
}

/// egui font data over a face's bytes, without copying them (CJK faces are 10–20 MB). Font
/// database faces are never dropped, so a system face's shared bytes are kept for good: leaking
/// one more reference to them costs a pointer and gives egui the `'static` slice it needs.
pub fn font_data(face: &figcraft_text::Face) -> FontData {
    let bytes: &'static [u8] = match &face.bytes {
        figcraft_text::fontdb::FontBytes::Static(b) => b,
        figcraft_text::fontdb::FontBytes::Shared(v) => Box::leak(Box::new(v.clone())).as_slice(),
    };
    let mut data = FontData::from_static(bytes);
    data.index = face.index;
    data
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn c32(c: figcraft_color::Color) -> Color32 {
    let [r, g, b, a] = c.to_rgba8();
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn screen_conversion_id() -> egui::Id {
    egui::Id::new("figcraft_screen_conversion")
}

/// Remember the colour profile of the document on screen (once per frame): document colours drawn
/// by the UI ([`doc_c32`]) go from it to the screen's sRGB.
pub fn set_doc_profile(ctx: &Context, profile: figcraft_color::ColorProfile) {
    let conv = profile.conversion_to(figcraft_color::ColorProfile::Srgb);
    ctx.data_mut(|d| d.insert_temp(screen_conversion_id(), conv));
}

/// A document colour (a paint, swatch, background) for the screen: converted from the document's
/// colour profile to sRGB. UI chrome colours use [`c32`].
pub fn doc_c32(ctx: &Context, c: figcraft_color::Color) -> Color32 {
    let conv = ctx.data(|d| d.get_temp::<figcraft_color::ProfileConversion>(screen_conversion_id()));
    c32(conv.map_or(c, |k| k.apply(c)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_font_draws_every_mac_shortcut_glyph() {
        figcraft_text::scan_system_fonts();
        let lacking: String = KEY_SYMBOLS.chars().filter(|c| figcraft_text::face_with_char(*c, &KEY_FAMILIES).is_none()).collect();
        if !lacking.is_empty() {
            eprintln!("no system face here draws {lacking}; skipping");
            return;
        }
        let ctx = Context::default();
        install_fonts(&ctx, true, 0);
        let input =
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0))), ..Default::default() };
        ctx.run_ui(input, |_| {}).textures_delta.clear();
        ctx.fonts_mut(|f| {
            for c in KEY_SYMBOLS.chars() {
                for font in [regular(11.0), semibold(11.0)] {
                    assert!(f.has_glyphs(&font, &c.to_string()), "UI font draws {c:?}");
                }
            }
        });
    }
}
