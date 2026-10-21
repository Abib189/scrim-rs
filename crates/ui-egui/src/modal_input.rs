//! Stage K2: a modal dialog (Settings) keeps the wheel, trackpad scroll and pinch to itself.
//!
//! - egui knows the modal layer, but the canvas reads the scroll and the zoom from the frame's input
//!   once its own response is hovered, and that still held under Settings. So the readers of raw
//!   scroll ask here first: the canvas's pan and zoom, and follow's "the person scrolled the canvas".
//! - A dialog that isn't modal (Missing fonts, Local variables, Export, a confirmation) covers only
//!   itself: egui's hover test already keeps its wheel from the canvas, and follow checks the layer
//!   under the pointer too.

use egui::{Context, LayerId, Order, Pos2};

/// A modal dialog is open: nothing behind it may take the wheel, the scroll or a pinch.
pub fn behind_modal(ctx: &Context) -> bool {
    ctx.memory(|m| !m.is_above_modal_layer(LayerId::background()))
}

/// The canvas at `pos` is covered: by a modal anywhere, or by a dialog, menu or popup drawn above it
/// there.
pub fn canvas_covered(ctx: &Context, pos: Pos2) -> bool {
    behind_modal(ctx) || ctx.layer_id_at(pos).is_some_and(|l| l.order != Order::Background)
}
