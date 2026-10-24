//! What an edit did to each layer (Fix Z1), for the AI panel's run card: created, edited, moved or
//! deleted, with the layer's name as it was then (so a deleted layer is still named afterwards).
//! Names are the person's own content; nothing else is kept.

use std::sync::Arc;

use figcraft_doc::{Document, NodeId, NodeKind};
use serde_json::Value;

use crate::{Result, Session};

impl Session {
    /// `command.batch`: its steps are logged as a batch's, and the batch with the label it was
    /// given (`given`, `None` when it had none), so the run card can say an agent's own sentence.
    pub(crate) fn logged_batch(&mut self, label: &str, given: Option<String>, cmds: &[(String, Value)]) -> Result<Vec<Value>> {
        self.agents.batch_depth += 1;
        let r = self.batch(label, cmds);
        self.agents.batch_depth = self.agents.batch_depth.saturating_sub(1);
        self.agents.batch_label = given;
        r
    }
}

/// What happened to one layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Touch {
    Created,
    Edited,
    /// Moved into another parent, named as it was then.
    Moved {
        into: String,
    },
    Deleted,
}

/// One layer an edit touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Touched {
    pub id: NodeId,
    pub name: String,
    pub touch: Touch,
    /// Its parent (after the edit; before it for a deleted layer).
    pub parent: Option<NodeId>,
    /// A frame, section or group: a container a row names on its own.
    pub container: bool,
    /// A frame (Fix AB2: the card's "Created 2 frames and 36 layers").
    pub frame: bool,
    /// Right on a page.
    pub top: bool,
}

fn name_of(doc: &Document, id: NodeId) -> String {
    doc.node(id).map(|n| n.name.trim().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| "a layer".into())
}

fn on_page(doc: &Document, parent: Option<NodeId>) -> bool {
    parent.and_then(|p| doc.node(p)).is_some_and(|p| matches!(p.kind, NodeKind::Page { .. }))
}

/// What happened to each layer `ids` names, from `before` to `after`. A layer whose only change is its
/// list of children (something was added or removed inside it) isn't counted as edited.
pub fn touched(before: &Document, after: &Document, ids: &[NodeId]) -> Vec<Touched> {
    let mut out = Vec::new();
    for &id in ids {
        let (b, a) = (before.node(id), after.node(id));
        let (doc, node, touch) = match (b, a) {
            (None, Some(n)) => (after, n, Touch::Created),
            (Some(n), None) => (before, n, Touch::Deleted),
            (Some(bn), Some(an)) => {
                let same = before.node_arc(id).zip(after.node_arc(id)).is_some_and(|(x, y)| Arc::ptr_eq(x, y));
                let mut x = bn.clone();
                x.children = an.children.clone();
                if same || x == *an {
                    continue;
                }
                let touch = if bn.parent != an.parent {
                    Touch::Moved { into: an.parent.map(|p| name_of(after, p)).unwrap_or_default() }
                } else {
                    Touch::Edited
                };
                (after, an, touch)
            }
            (None, None) => continue,
        };
        out.push(Touched {
            id,
            name: name_of(doc, id),
            touch,
            parent: node.parent,
            container: matches!(node.kind, NodeKind::Frame | NodeKind::Section { .. } | NodeKind::Group),
            frame: matches!(node.kind, NodeKind::Frame),
            top: on_page(doc, node.parent),
        });
    }
    out
}
