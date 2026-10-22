//! Fix S3: nothing is drawn under macOS's window buttons.
//! - The inset is one function of the title bar: 76 pt for macOS's full-size content view
//!   (`TitleBar::Inset`), a margin otherwise.
//! - Drawn with each title bar, on the home screen and in the editor, light and dark: inside the
//!   window buttons' area only backgrounds that span the window are painted (no tab, icon or text).
//! - A secondary window opens with the main window's title bar, so the same inset applies to it.

use egui::{Rect, Shape, pos2};
use serde_json::json;

use super::window_buttons_inset;
use crate::TitleBar;
use crate::layers_drag_tests::Ui;

/// Where macOS draws its three window buttons over a full-size content view, with a margin: the
/// real window's captures in `docs/scrim-ports/window-buttons/` show them from x = 9 to 68 and
/// y = 9 to 23. Fixed here, not taken from the code under test.
fn buttons_area() -> Rect {
    Rect::from_min_max(pos2(0.0, 0.0), pos2(72.0, 30.0))
}

/// Shapes painted inside `area` that are smaller than the window (a tab, an icon, text).
fn under(shapes: Vec<egui::epaint::ClippedShape>, area: Rect) -> Vec<String> {
    fn walk(s: &Shape, area: Rect, out: &mut Vec<String>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, area, out)),
            Shape::Noop => {}
            s => {
                let b = s.visual_bounding_rect();
                if b.is_positive() && b.intersects(area) && b.width() < 1000.0 {
                    out.push(format!("{:?} at {b:?}", std::mem::discriminant(s)));
                }
            }
        }
    }
    let mut out = Vec::new();
    for c in shapes {
        walk(&c.shape, area, &mut out);
    }
    out
}

fn window(bar: TitleBar, dark: bool, home: bool) -> Ui {
    let mut s = figcraft_engine::Session::new();
    s.execute("frame.create", &json!({"x": 0, "y": 0, "width": 200, "height": 200})).unwrap();
    s.prefs.theme = if dark { "dark" } else { "light" }.into();
    let mut u = Ui::new(s);
    u.app.ui.title_bar = bar;
    u.app.ui.home = home;
    for _ in 0..4 {
        u.frame(vec![]);
    }
    u
}

#[test]
fn the_inset_leaves_room_for_macos_window_buttons_and_only_there() {
    assert_eq!(window_buttons_inset(TitleBar::Inset), 76.0, "the buttons end at x = 68, and a margin");
    assert_eq!(window_buttons_inset(TitleBar::Custom), 8.0);
    assert_eq!(window_buttons_inset(TitleBar::Native), 8.0);
}

#[test]
fn nothing_is_drawn_under_the_window_buttons_on_home_or_in_the_editor_in_either_theme() {
    for dark in [false, true] {
        for home in [true, false] {
            let mut u = window(TitleBar::Inset, dark, home);
            let out = u.frame_output(vec![]);
            let found = under(out.shapes, buttons_area());
            assert!(found.is_empty(), "dark {dark}, home {home}: drawn under the window buttons: {found:?}");
        }
    }
}

#[test]
fn without_the_inset_the_home_tab_would_sit_under_the_buttons() {
    // The test sees what it guards: with a native title bar the strip starts at the margin, so
    // the Home tab is inside the buttons' area.
    let mut u = window(TitleBar::Native, false, true);
    let out = u.frame_output(vec![]);
    assert!(!under(out.shapes, buttons_area()).is_empty());
}

#[test]
fn a_secondary_window_opens_with_the_same_title_bar() {
    let mut u = window(TitleBar::Inset, false, false);
    u.ctx.set_embed_viewports(false);
    let ctx = u.ctx.clone();
    crate::windows::open(&mut u.app, &ctx, Some(0), None, None).unwrap();
    let w = u.app.windows.list.last().expect("a second window");
    assert_eq!(w.title_bar(), TitleBar::Inset, "it draws the same tab strip, with the same inset");
}
