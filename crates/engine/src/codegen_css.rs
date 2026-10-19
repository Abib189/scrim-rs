//! CSS for what the HTML export draws beyond plain boxes (split out of `codegen.rs`): shapes as
//! inline SVG, image paints as data URIs, gradients, and masks as `clip-path`.

use super::*;

/// Whether `n` is a shape CSS can't draw (a path, polygon, star, line or boolean) that the HTML
/// export draws as inline SVG instead. The SVG carries the layer's turn and flip in page space, so
/// the layer's box itself is not turned.
pub(crate) fn svg_drawn(doc: &Document, n: &Node) -> bool {
    let shape =
        matches!(n.kind, NodeKind::Vector { .. } | NodeKind::Polygon { .. } | NodeKind::Star { .. } | NodeKind::Line | NodeKind::BooleanOp { .. });
    // An ellipse is CSS (`border-radius: 50%`) unless it is an arc or a ring.
    let arc =
        matches!(n.kind, NodeKind::Ellipse { start, sweep, inner } if start.abs() > 1e-9 || sweep < std::f64::consts::TAU - 1e-9 || inner > 1e-9);
    let [a, b, c, d, _, _] = doc.world_transform(n.id).as_coeffs();
    (shape || arc) && (a * d - b * c).abs() > 1e-9
}

/// The inline SVG drawing `n` (see [`svg_drawn`]), placed over the layer's box: it covers the
/// layer's export bounds, which strokes and effects can widen.
pub(super) fn inline_svg(doc: &Document, n: &Node) -> Option<String> {
    let bounds = figcraft_render::export_bounds(doc, n.id)?;
    let origin = doc.world_transform(n.id).as_coeffs();
    let svg = figcraft_svg::export_svg(doc, &[n.id], &figcraft_svg::SvgOptions { bounds: Some(bounds), ..Default::default() })?;
    let body =
        svg.trim_start_matches(|c| c != '<').strip_prefix("<?xml").map_or(svg.as_str(), |rest| rest.split_once("?>").map_or(rest, |(_, r)| r)).trim();
    let style =
        format!(" style=\"position: absolute; left: {}px; top: {}px; overflow: visible\"", num(bounds.x0 - origin[4]), num(bounds.y0 - origin[5]));
    Some(body.replacen("<svg", &format!("<svg{style}"), 1))
}

/// The CSS `clip-path` for mask layer `m` in `parent`: an unturned rectangle (with its corner
/// radii) or ellipse, in the parent's coordinates. Other masks (paths, turned layers) have none.
pub(super) fn mask_clip(parent: &Node, m: &Node) -> Option<String> {
    let [a, b, c, d, x, y] = m.transform.as_coeffs();
    if b.abs() > 1e-9 || c.abs() > 1e-9 || (a - 1.0).abs() > 1e-9 || (d - 1.0).abs() > 1e-9 {
        return None;
    }
    let (w, h) = (m.width, m.height);
    match m.kind {
        NodeKind::Rectangle | NodeKind::Frame => {
            let r = m.corners.radii;
            let round = if r.iter().any(|v| *v > 0.0) {
                format!(" round {}px {}px {}px {}px", num(r[0]), num(r[1]), num(r[2]), num(r[3]))
            } else {
                String::new()
            };
            Some(format!("inset({}px {}px {}px {}px{round})", num(y), num(parent.width - x - w), num(parent.height - y - h), num(x)))
        }
        NodeKind::Ellipse { sweep, inner, .. } if sweep >= std::f64::consts::TAU - 1e-9 && inner <= 1e-9 => {
            Some(format!("ellipse({}px {}px at {}px {}px)", num(w / 2.0), num(h / 2.0), num(x + w / 2.0), num(y + h / 2.0)))
        }
        _ => None,
    }
}

/// An image paint as a background layer: the image inlined as a data URI (the page needs no other
/// file), placed as its scale mode says. Crop is shown as Fill and rotation is not exported.
pub(super) fn css_image(doc: &Document, img: &figcraft_color::ImagePaint) -> String {
    use base64::Engine as _;
    use figcraft_color::ImageScaleMode as Mode;
    let Some(data) = doc.images.get(&img.image) else { return "url(image.png) center / cover no-repeat".into() };
    let b = data.bytes.as_slice();
    let mime = if b.starts_with(b"\x89PNG") {
        "image/png"
    } else if b.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg"
    } else if b.starts_with(b"GIF8") {
        "image/gif"
    } else if b.get(8..12) == Some(b"WEBP".as_slice()) {
        "image/webp"
    } else {
        "application/octet-stream"
    };
    let url = format!("url(data:{mime};base64,{})", base64::engine::general_purpose::STANDARD.encode(b));
    match img.scale_mode {
        Mode::Fill | Mode::Crop => format!("{url} center / cover no-repeat"),
        Mode::Fit => format!("{url} center / contain no-repeat"),
        Mode::Tile => {
            let (w, h) = (f64::from(data.width) * img.scale, f64::from(data.height) * img.scale);
            format!("{url} 0 0 / {}px {}px repeat", num(w), num(h))
        }
    }
}

pub(super) fn css_gradient(g: &figcraft_color::Gradient, w: f64, h: f64) -> String {
    let stops = g.sorted_stops().iter().map(|s| format!("{} {}%", s.color.to_css(), (s.position * 100.0).round())).collect::<Vec<_>>().join(", ");
    let (dx, dy) = ((g.to.0 - g.from.0) * w, (g.to.1 - g.from.1) * h);
    let angle = if dx == 0.0 && dy == 0.0 { 180.0 } else { dx.atan2(-dy).to_degrees().rem_euclid(360.0) };
    let pct = |v: f64| format!("{}%", num(v * 100.0));
    let at = format!("at {} {}", pct(g.from.0), pct(g.from.1));
    match g.kind {
        figcraft_color::GradientKind::Linear => format!("linear-gradient({}deg, {stops})", num(angle)),
        figcraft_color::GradientKind::Radial | figcraft_color::GradientKind::Diamond => {
            let (rx, ry) = (((g.width.0 - g.from.0) * w).hypot((g.width.1 - g.from.1) * h), dx.hypot(dy));
            let rel = |r: f64, side: f64| if side > 0.0 { pct(r / side) } else { "50%".into() };
            format!("radial-gradient({} {} {at}, {stops})", rel(rx, w), rel(ry, h))
        }
        figcraft_color::GradientKind::Angular => format!("conic-gradient(from {}deg {at}, {stops})", num(angle)),
    }
}
