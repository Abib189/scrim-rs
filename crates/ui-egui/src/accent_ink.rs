//! The accent where code paints it (Stage G, `docs/design/accent-sweep.md`): the person's chosen
//! accent, live, for everything interactive.
//! - **In the panels:** [`fill`] (buttons, the active tool, chips, edges and rings), [`on`] (the
//!   label or icon on a fill, black or white by contrast), and [`text`] (accent-coloured text at
//!   4.5:1). They are the theme's tokens (`Tokens::accent_fill` / `accent_on` / `accent_text`).
//! - **On the canvas:** [`overlay`] (selection, handles, marquee, paths, builder regions) and
//!   [`overlay_on`]. The overlay is the chosen colour where it reads at 3:1 on the page behind it,
//!   its lightness moved where it doesn't ([`for_page`]), the hue kept.
//! - **Live, without arguments:** [`install`] (from `theme::apply_with`, every frame) keeps the
//!   tokens, and [`set_page`] (from the window, every frame, before the canvas) the page's colour.
//!   The UI runs on one thread, so they are kept per thread: a test's window has its own.
//!   Changing the accent repaints everything on the next frame; nothing is copied at startup.
//! - **Not for the brand mark** (`theme::BRAND`) or semantic colours: see the plan.

use std::cell::Cell;

use egui::Color32;

use crate::theme::Tokens;
use crate::theme::accent::{DEFAULT_ACCENT, INK, contrast, from_hsl, luminance, to_hsl};

/// What an overlay needs against the page behind it (non-text: 3:1).
pub const OVERLAY_CONTRAST: f64 = 3.0;

#[derive(Clone, Copy)]
struct Live {
    fill: Color32,
    on: Color32,
    text: Color32,
    overlay: Color32,
    base: Color32,
}

const DEFAULT: Live = Live { fill: DEFAULT_ACCENT, on: INK, text: DEFAULT_ACCENT, overlay: DEFAULT_ACCENT, base: DEFAULT_ACCENT };

thread_local! {
    static LIVE: Cell<Live> = const { Cell::new(DEFAULT) };
    static PAGE: Cell<Option<Color32>> = const { Cell::new(None) };
}

/// Keep `t`'s accent for this frame (`theme::apply_with`).
pub fn install(t: &Tokens) {
    let page = PAGE.with(Cell::get).unwrap_or(t.canvas);
    LIVE.with(|l| {
        l.set(Live { fill: t.accent_fill, on: t.accent_on, text: t.accent_text, overlay: for_page(t.accent_base, page), base: t.accent_base })
    });
}

/// The canvas shows `page` (its background) this frame: the overlay accent is for it.
pub fn set_page(page: Color32) {
    PAGE.with(|p| p.set(Some(page)));
    LIVE.with(|l| {
        let mut v = l.get();
        v.overlay = for_page(v.base, page);
        l.set(v);
    });
}

/// [`set_page`] with the background of the page `app` shows (the theme's canvas without one).
pub fn set_page_of(app: &crate::FigApp, ctx: &egui::Context) {
    let page = app.session.ds().ok().and_then(|ds| match ds.doc.node(ds.page).map(|n| &n.kind) {
        Some(figcraft_doc::NodeKind::Page { background, .. }) => Some(crate::theme::doc_c32(ctx, *background)),
        _ => None,
    });
    set_page(page.unwrap_or_else(|| Tokens::get(ctx).canvas));
}

/// The accent fill: primary buttons, the active tool, chips, selected rings and edges in the panels.
pub fn fill() -> Color32 {
    LIVE.with(Cell::get).fill
}

/// The label or icon on [`fill`]: black or white, whichever reads (4.5:1).
pub fn on() -> Color32 {
    LIVE.with(Cell::get).on
}

/// Accent-coloured text (4.5:1 on the panel).
pub fn text() -> Color32 {
    LIVE.with(Cell::get).text
}

/// The canvas overlays' accent, for the page shown.
pub fn overlay() -> Color32 {
    LIVE.with(Cell::get).overlay
}

/// A label or mark on [`overlay`]: black or white, whichever reads better.
pub fn overlay_on() -> Color32 {
    on_of(overlay())
}

/// Black or white on `c`, whichever has the higher contrast.
pub fn on_of(c: Color32) -> Color32 {
    if contrast(Color32::WHITE, c) >= contrast(INK, c) { Color32::WHITE } else { INK }
}

/// `base` as a canvas overlay on `page`: itself where it reads at [`OVERLAY_CONTRAST`], else its
/// lightness moved away from the page's (darker on a light page, lighter on a dark one) until it
/// does, the hue and saturation kept.
pub fn for_page(base: Color32, page: Color32) -> Color32 {
    reaching(base, page, OVERLAY_CONTRAST)
}

/// The live accent as text on the canvas's `page` (a selected frame title): 4.5:1 there.
pub fn text_on_page(page: Color32) -> Color32 {
    reaching(LIVE.with(Cell::get).base, page, 4.5)
}

/// `base` with its lightness moved until it reaches `ratio` on `page` (itself if it already does).
fn reaching(base: Color32, page: Color32, ratio: f64) -> Color32 {
    if contrast(base, page) >= ratio {
        return base;
    }
    let (h, s, l) = to_hsl(base);
    let darker = luminance(page) > 0.18;
    let mut lit = l;
    for _ in 0..100 {
        lit = if darker { (lit - 0.01).max(0.0) } else { (lit + 0.01).min(1.0) };
        let c = from_hsl(h, s, lit);
        if contrast(c, page) >= ratio {
            return c;
        }
    }
    from_hsl(h, s, lit)
}

#[cfg(test)]
#[path = "accent_ink_tests.rs"]
mod tests;
