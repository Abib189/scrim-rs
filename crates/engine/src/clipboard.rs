//! The session's own clipboard (split out of `lib.rs`).

use std::sync::Arc;

use figcraft_doc::{Document, NodeId};
use serde_json::Value;

/// Clipboard contents (internal; the UI mirrors to the system clipboard).
#[derive(Clone, Debug, Default)]
pub struct Clipboard {
    /// Copied subtrees: (root, nodes) in z-order.
    pub nodes: Vec<(NodeId, Vec<figcraft_doc::Node>)>,
    pub images: std::collections::BTreeMap<String, figcraft_doc::ImageData>,
    /// Copied properties (Copy properties / Paste properties).
    pub props: Option<Value>,
    /// The page-space bounds of the copied selection.
    pub bounds: Option<figcraft_geom::Rect>,
    pub source_doc: u64,
    /// The document copied from (pastes into other files follow its mains) and its file path.
    pub source: Option<Arc<Document>>,
    pub source_path: Option<String>,
}
