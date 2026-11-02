//! Files dropped on the window: their contents, and the overlay saying what a drop does.

/// The contents of a dropped file that has no file on disk to read by path.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn dropped_bytes(f: &egui::DroppedFileHandle) -> Option<Vec<u8>> {
    f.bytes().ok()
}

/// The contents of a dropped file that has no file on disk to read by path.
#[cfg(target_arch = "wasm32")]
pub(crate) fn dropped_bytes(_f: &egui::DroppedFileHandle) -> Option<Vec<u8>> {
    // Browser drops arrive asynchronously; not supported yet.
    None
}

/// While files are dragged over the window: a tinted, outlined `rect` with `label` in its middle,
/// over everything else, saying what a drop does there.
pub(crate) fn drop_overlay(ctx: &egui::Context, rect: egui::Rect, label: &str) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let t = crate::theme::Tokens::get(ctx);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop_overlay")));
    let area = rect.shrink(4.0);
    painter.rect_filled(area, egui::CornerRadius::same(8), t.accent.gamma_multiply(0.12));
    painter.rect_stroke(area, egui::CornerRadius::same(8), egui::Stroke::new(2.0, t.accent), egui::StrokeKind::Inside);
    let galley = painter.layout_no_wrap(label.to_owned(), crate::theme::semibold(14.0), t.text);
    let pill = egui::Rect::from_center_size(area.center(), galley.size() + egui::vec2(32.0, 16.0));
    painter.rect_filled(pill, egui::CornerRadius::same(8), t.panel);
    painter.rect_stroke(pill, egui::CornerRadius::same(8), egui::Stroke::new(1.0, t.accent), egui::StrokeKind::Inside);
    painter.galley(pill.center() - galley.size() / 2.0, galley, t.text);
    // Hovering sends no pointer events on every platform: keep the overlay current.
    ctx.request_repaint();
}
