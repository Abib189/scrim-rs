//! A refused batch's fixes take time and space linear in the batch (upstream FigCraft 08173cc5,
//! audit #1063 COL-2): the server restores each deleted layer once, walking each subtree once,
//! and looks up and sends each document-level fix once. Before, a batch deleting every layer of a
//! deep chain restored each layer once per deleted ancestor, quadratic in the chain, on the hub
//! thread every connection shares.

use std::collections::BTreeMap;

use figcraft_doc::{Document, NodeId, NodeKind};

use crate::change::{Change, Json, Prop};
use crate::message::{ClientMsg, ServerMsg};
use crate::{Client, Limits, Outgoing, Server};

#[test]
fn a_refused_deep_delete_restores_each_layer_once() {
    let mut d = Document::new("Shared");
    let page = d.first_page().unwrap();
    let mut chain = Vec::new();
    let mut parent = page;
    for i in 0..300 {
        parent = d.create(parent, NodeKind::Frame, &format!("F{i}"), 0.0, 0.0, 10.0, 10.0).unwrap();
        chain.push(parent);
    }
    let mut s = Server::new(d, "tok", Limits::default());
    let ClientMsg::Hello(h) = Client::hello("f", "tok", "Ada", "#FF8800") else { panic!() };
    let (me, _) = s.join(h).unwrap();
    // Delete the whole chain, and one change that can't apply, so the batch is refused.
    let mut changes: Vec<Change> = chain.iter().map(|id| Change::Delete { id: *id }).collect();
    changes.push(Change::Set { id: NodeId(4242), prop: Prop::Name, value: Json(serde_json::json!("x")) });
    let out = s.receive(me, ClientMsg::Changes { base_seq: s.seq(), batch_id: 1, changes });
    let [Outgoing::To(_, ServerMsg::Reject { fixes, .. })] = out.as_slice() else { panic!("not refused: {out:?}") };
    let mut restored: BTreeMap<NodeId, usize> = BTreeMap::new();
    for f in fixes {
        if let Change::Create { id, .. } = f {
            *restored.entry(*id).or_default() += 1;
        }
    }
    assert_eq!(restored.len(), chain.len(), "every deleted layer comes back");
    let most = restored.values().max().copied().unwrap_or(0);
    let total: usize = restored.values().sum();
    assert_eq!((most, total), (1, chain.len()), "each layer once: {total} creates for {} layers", chain.len());
}
