//! Auto layout: a frame's or instance's `autoLayout` (direction, spacing, padding, alignment,
//! wrapping, its own hug or fixed size) and how each layer sits in an auto layout parent
//! (`stretchWidth` / `stretchHeight` fill, `absolutePos`, minimum and maximum sizes).
//!
//! The README's `align` is the horizontal alignment of the children and `valign` the vertical
//! one, whatever the direction: in a horizontal layout `align` is the main axis, in a vertical
//! one `valign` is; `Distribute` spreads the children (space between). `fixWidth: false` hugs
//! the contents, or fills the parent when the layer also stretches its width.

use figcraft_doc::{AutoLayout, ContentAlign, CrossAlign, LayoutMode, MainAlign, Node, NodeKind, Sizing, TextAutoResize};
use serde_json::Value;

use super::Ctx;
use super::json::{self, MAX_SIZE, flag, int, num};
use super::layers::positive;
use crate::import_report::Severity;

/// A `HorizontalAlignment` / `VerticalAlignment` (0 start, 1 centre, 2 end, 3 distribute) on the
/// main axis.
fn main_align(v: i64) -> MainAlign {
    match v {
        1 => MainAlign::Center,
        2 => MainAlign::Max,
        3 => MainAlign::SpaceBetween,
        _ => MainAlign::Min,
    }
}

impl Ctx<'_> {
    /// An `AutoLayout` object.
    pub(super) fn auto_layout(&mut self, a: &Value, path: &str) -> AutoLayout {
        let vertical = flag(a, "vertical") == Some(true);
        let (h, v) = (int(a, "align").unwrap_or(0), int(a, "valign").unwrap_or(0));
        let (main, cross) = if vertical { (v, h) } else { (h, v) };
        let cross_align = match cross {
            1 => CrossAlign::Center,
            2 => CrossAlign::Max,
            3 => {
                self.warn(Severity::Approximated, path, "Distributing children across the auto layout's cross axis was imported as start-aligned.");
                CrossAlign::Min
            }
            _ => CrossAlign::Min,
        };
        let baseline = flag(a, "textBaseline") == Some(true);
        if baseline && vertical {
            self.warn(Severity::Approximated, path, "Text baseline alignment in a vertical auto layout was not imported.");
        }
        let padding = json::thickness(a, "padding").map_or([0.0; 4], |[l, t, r, b]| [t, r, b, l]);
        if a.get("padding").is_some() && json::thickness(a, "padding").is_none() {
            self.warn(Severity::Dropped, path, "The auto layout's padding couldn't be read and was not imported.");
        }
        if flag(a, "reverseZIndex") == Some(true) {
            self.warn(
                Severity::Dropped,
                path,
                "The auto layout's stacking option was not imported: the README names it both “reverse z-index” and “last on top”.",
            );
        }
        AutoLayout {
            mode: if vertical { LayoutMode::Vertical } else { LayoutMode::Horizontal },
            wrap: flag(a, "wrap") == Some(true),
            gap: num(a, "spacing").unwrap_or(0.0).clamp(-MAX_SIZE, MAX_SIZE),
            cross_gap: num(a, "wrapSpacing").unwrap_or(0.0).clamp(-MAX_SIZE, MAX_SIZE),
            padding,
            main_align: main_align(main),
            cross_align: if baseline && !vertical { CrossAlign::Baseline } else { cross_align },
            content_align: ContentAlign::Auto,
            strokes_included: flag(a, "strokesIncluded") == Some(true),
            reverse_z: false,
            ..AutoLayout::default()
        }
    }

    /// How `n` sizes and sits in its parent (`parent_auto`: the parent has auto layout).
    pub(super) fn child_layout(&mut self, n: &mut Node, j: &Value, parent_auto: bool, path: &str) {
        let cl = &mut n.child_layout;
        // A text's resize behaviour is its sizing: flexible text hugs both ways, auto height its height.
        if let NodeKind::Text(t) = &n.kind {
            match t.auto_resize {
                TextAutoResize::WidthAndHeight => (cl.sizing_h, cl.sizing_v) = (Sizing::Hug, Sizing::Hug),
                TextAutoResize::Height => cl.sizing_v = Sizing::Hug,
                _ => {}
            }
        }
        let own = j.get("autoLayout").filter(|_| n.layout.is_some());
        let stretch_w = flag(j, "stretchWidth") == Some(true);
        let stretch_h = flag(j, "stretchHeight") == Some(true);
        if let Some(a) = own {
            cl.sizing_h = if flag(a, "fixWidth") == Some(true) { Sizing::Fixed } else { Sizing::Hug };
            cl.sizing_v = if flag(a, "fixHeight") == Some(true) { Sizing::Fixed } else { Sizing::Hug };
        }
        if parent_auto {
            if stretch_w {
                cl.sizing_h = Sizing::Fill;
            }
            if stretch_h {
                cl.sizing_v = Sizing::Fill;
            }
            cl.absolute = flag(j, "absolutePos") == Some(true);
        } else if flag(j, "absolutePos") == Some(true) || stretch_w || stretch_h {
            self.warn(Severity::Info, path, "The layer's auto layout child settings have no effect: its parent has no auto layout.");
        }
        let clamp = |x: f64| x.min(MAX_SIZE);
        cl.min_w = positive(j, "minWidth").map(clamp);
        cl.max_w = positive(j, "maxWidth").map(clamp);
        cl.min_h = positive(j, "minHeight").map(clamp);
        cl.max_h = positive(j, "maxHeight").map(clamp);
    }
}
