//! Duplicate (`edit.duplicate`): Cmd+D, the Edit menu, an agent's command, and the copy an
//! Option-drag drops (Fix AE). The drag shows the copies [`copies`] makes, placed as [`place`]
//! places them, and its release runs this command: one way to duplicate, with Cmd+D's names.

use figcraft_doc::*;
use figcraft_geom::Affine;
use serde_json::{Value, json};

use super::edit::{Duplicated, roots, z_sorted};
use crate::params::*;
use crate::{Result, Session, bad};

/// `{ids?, dx?, dy?, parent?, index?}`. With `parent`, the copies go into that layer keeping their
/// place on the canvas, at `index` among its other layers when given.
pub(super) fn duplicate(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = req_targets("edit.duplicate", s, p)?;
    let rename = s.prefs.rename_duplicates;
    let offset = (num(p, "dx"), num(p, "dy"));
    let into = opt_id("edit.duplicate", p, "parent")?;
    let index = num(p, "index").map(|i| i.max(0.0) as usize);
    let ds = s.ds()?;
    let ids = z_sorted(&ds.doc, &roots(&ds.doc, &ids));
    let explicit = offset.0.is_some() || offset.1.is_some();
    let repeat = ds.last_duplicate.as_ref().filter(|d| !explicit && d.is_selection(&ids)).and_then(|d| d.offset(&ds.doc));
    let doc = s.doc_mut()?;
    let out = copies(doc, &ids, offset, repeat, rename)?;
    if let Some(parent) = into {
        place(doc, &out, parent, index)?;
    }
    s.set_selection(out.clone());
    s.ds_mut()?.last_duplicate = Some(Duplicated { sources: ids, copies: out.clone() });
    Ok(json!({"ids": out}))
}

/// A deep copy of each of `ids` (top-level layers, in paint order) directly above it in the same
/// parent. It is offset by `offset` in that parent's space, else by `repeat` on the page; with
/// neither, top-level frames go to the right with a 100 gap and everything else stays in place.
/// With `rename`, the number ending a copy's name goes up by one (else " 2" is added).
pub(crate) fn copies(
    doc: &mut Document,
    ids: &[NodeId],
    offset: (Option<f64>, Option<f64>),
    repeat: Option<(f64, f64)>,
    rename: bool,
) -> Result<Vec<NodeId>> {
    let mut out = Vec::new();
    for i in ids.iter().copied() {
        let n = doc.get(i)?;
        let parent = n.parent.ok_or_else(|| bad("edit.duplicate", "pages are duplicated with page.duplicate"))?;
        let top_frame = doc.node(parent).is_some_and(Node::is_page) && n.kind.is_frame_like();
        let idx = doc.index_in_parent(i).map(|x| x + 1);
        let w = n.width;
        let nid = doc.duplicate_into(i, parent, idx)?;
        let (dx, dy) = match offset {
            (Some(dx), dy) => (dx, dy.unwrap_or(0.0)),
            (None, Some(dy)) => (0.0, dy),
            _ if top_frame => (w + 100.0, 0.0),
            _ => (0.0, 0.0),
        };
        match repeat {
            Some((rx, ry)) => super::arrange::move_world(doc, nid, rx, ry)?,
            None => {
                let nn = doc.get_mut(nid)?;
                nn.transform = Affine::translate((dx, dy)) * nn.transform;
            }
        }
        if rename {
            let nn = doc.get_mut(nid)?;
            nn.name = bump_name(&nn.name);
        }
        out.push(nid);
    }
    Ok(out)
}

/// Put `layers` in `parent`, keeping their place on the canvas: at `index` among its other layers
/// (in their order), else on top of them, or where they are when it is their parent already.
pub(crate) fn place(doc: &mut Document, layers: &[NodeId], parent: NodeId, index: Option<usize>) -> Result<()> {
    for l in layers {
        if doc.get(*l)?.parent != Some(parent) {
            doc.reparent(*l, parent, None)?;
        }
    }
    if let Some(at) = index {
        let mut kids: Vec<NodeId> = doc.get(parent)?.children.iter().copied().filter(|c| !layers.contains(c)).collect();
        for (k, l) in layers.iter().enumerate() {
            kids.insert((at + k).min(kids.len()), *l);
        }
        doc.get_mut(parent)?.children = kids;
    }
    Ok(())
}

fn bump_name(n: &str) -> String {
    if let Some((base, num)) = n.rsplit_once(' ')
        && let Ok(k) = num.parse::<u64>()
        && let Some(next) = k.checked_add(1)
    {
        return format!("{base} {next}");
    }
    format!("{n} 2")
}
