//! The icon a layer row shows for its kind (and a boolean operation's icon), split out of the
//! layers panel.

use figcraft_doc::{Document, NodeId, NodeKind};

use crate::icons::Icon;

/// A boolean operation's icon.
pub fn op_icon(op: figcraft_doc::BoolOp) -> Icon {
    match op {
        figcraft_doc::BoolOp::Union => Icon::Union,
        figcraft_doc::BoolOp::Subtract => Icon::Subtract,
        figcraft_doc::BoolOp::Intersect => Icon::Intersect,
        figcraft_doc::BoolOp::Exclude => Icon::Exclude,
    }
}

pub fn node_icon(doc: &Document, id: NodeId) -> Icon {
    let Some(n) = doc.node(id) else { return Icon::Rectangle };
    // An operand combining by an operation other than its group's shows that operation.
    if let Some(op) = figcraft_engine::booleans::distinct_op(doc, id) {
        return op_icon(op);
    }
    match &n.kind {
        NodeKind::Page { .. } => Icon::Page,
        NodeKind::Frame if figcraft_engine::components::is_slot(doc, id) => Icon::Slot,
        NodeKind::Frame => {
            if let Some(l) = &n.layout {
                match l.mode {
                    figcraft_doc::LayoutMode::Vertical => Icon::LayoutV,
                    figcraft_doc::LayoutMode::Grid => Icon::LayoutGrid,
                    _ => Icon::LayoutH,
                }
            } else {
                Icon::Frame
            }
        }
        NodeKind::Group => Icon::Group,
        NodeKind::Section { .. } => Icon::Section,
        NodeKind::Rectangle => {
            if n.fills.iter().any(|f| matches!(f.kind, figcraft_color::PaintKind::Image(_))) {
                Icon::Image
            } else {
                Icon::Rectangle
            }
        }
        NodeKind::Ellipse { .. } => Icon::Ellipse,
        NodeKind::Polygon { .. } => Icon::Polygon,
        NodeKind::Star { .. } => Icon::Star,
        NodeKind::Line => {
            if n.stroke.cap_end.is_arrow() {
                Icon::Arrow
            } else {
                Icon::Line
            }
        }
        NodeKind::Vector { .. } => Icon::Vector,
        NodeKind::Text(_) => Icon::Text,
        NodeKind::BooleanOp { op } => op_icon(*op),
        NodeKind::Component { .. } => Icon::Component,
        NodeKind::ComponentSet { .. } => Icon::ComponentSet,
        NodeKind::Instance { .. } => Icon::Instance,
        NodeKind::Slice => Icon::Slice,
    }
}
