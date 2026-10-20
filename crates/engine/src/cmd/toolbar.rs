//! The toolbar's configuration (`docs/design/settings-and-connect.md`, C3b): which tools it shows, in
//! which order, and which share a group (a button with a menu of the others). Settings ▸ Toolbar
//! edits it, agents too, through these commands. A hidden tool keeps its shortcut, and Move can't
//! be hidden.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::CommandSpec;
use crate::params::*;
use crate::tools::ToolId;
use crate::{Result, Session, bad, spec};

/// One tool on the toolbar, shown or hidden.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEntry {
    pub tool: String,
    pub on: bool,
}

/// The groups, in order; a group of several tools is one button with a menu.
pub type Layout = Vec<Vec<ToolEntry>>;

/// The default toolbar: the groups the toolbar always had.
pub const DEFAULT: &[&[&str]] = &[
    &["move", "hand", "scale"],
    &["frame", "section", "slice"],
    &["rectangle", "line", "arrow", "ellipse", "polygon", "star"],
    &["pen", "pencil", "shapeBuilder"],
    &["text"],
    &["comment"],
];

/// The tool that can't be hidden.
pub const ALWAYS_SHOWN: &str = "move";

pub fn default_layout() -> Layout {
    DEFAULT.iter().map(|g| g.iter().map(|t| ToolEntry { tool: t.to_string(), on: true }).collect()).collect()
}

/// `layout` made valid: unknown and repeated tools dropped, empty groups dropped, tools it lacks
/// added (shown) in a group of their own at the end, and Move shown.
pub fn normalized(layout: &Layout) -> Layout {
    let known: Vec<&str> = DEFAULT.iter().flat_map(|g| g.iter().copied()).collect();
    let mut seen = std::collections::HashSet::new();
    let mut out: Layout = layout
        .iter()
        .map(|g| {
            g.iter()
                .filter_map(|e| {
                    let id = ToolId::from_id(&e.tool).map(ToolId::id).filter(|id| known.contains(id))?;
                    seen.insert(id).then(|| ToolEntry { tool: id.to_string(), on: e.on || id == ALWAYS_SHOWN })
                })
                .collect::<Vec<_>>()
        })
        .filter(|g| !g.is_empty())
        .collect();
    for id in known {
        if !seen.contains(id) {
            out.push(vec![ToolEntry { tool: id.to_string(), on: true }]);
        }
    }
    out
}

/// The toolbar the preferences describe (the default when they say nothing).
pub fn layout(prefs: &crate::Prefs) -> Layout {
    if prefs.settings.toolbar.is_empty() { default_layout() } else { normalized(&prefs.settings.toolbar) }
}

fn state(s: &Session) -> Value {
    let groups: Vec<Value> = layout(&s.prefs)
        .iter()
        .map(|g| {
            Value::Array(
                g.iter()
                    .map(|e| json!({"tool": e.tool, "on": e.on, "label": ToolId::from_id(&e.tool).map(ToolId::label), "shortcut": ToolId::from_id(&e.tool).map(ToolId::shortcut)}))
                    .collect(),
            )
        })
        .collect();
    json!({"groups": groups, "isDefault": layout(&s.prefs) == default_layout()})
}

/// Where `tool` is: (group, index).
fn find(l: &Layout, tool: &str) -> Option<(usize, usize)> {
    l.iter().enumerate().find_map(|(g, group)| group.iter().position(|e| e.tool == tool).map(|i| (g, i)))
}

fn tool_param(cmd: &str, p: &Value, key: &str) -> Result<String> {
    let t = string(p, key).ok_or_else(|| bad(cmd, format!("`{key}` is required (a tool id: move, frame, rectangle, …)")))?;
    ToolId::from_id(&t).map(|t| t.id().to_string()).ok_or_else(|| bad(cmd, "unknown tool"))
}

/// Change the layout with `f`, keep it (normalized) and answer the new state.
fn edit(s: &mut Session, cmd: &str, f: impl FnOnce(&mut Layout) -> std::result::Result<(), String>) -> Result<Value> {
    let mut l = layout(&s.prefs);
    f(&mut l).map_err(|why| bad(cmd, &why))?;
    let l = normalized(&l);
    s.prefs.settings.toolbar = if l == default_layout() { Vec::new() } else { l };
    Ok(state(s))
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!("toolbar.state", "Toolbar", "{} → {groups: [[{tool, on, label, shortcut}]], isDefault}", false, |s, _| Ok(state(s))),
        spec!(
            "toolbar.toggle",
            "Show or hide a toolbar tool",
            "{tool, on? (default: the opposite)} → the toolbar  (a hidden tool keeps its shortcut; Move can't be hidden)",
            false,
            |s, p| {
                let cmd = "toolbar.toggle";
                let tool = tool_param(cmd, p, "tool")?;
                let on = boolean(p, "on");
                edit(s, cmd, |l| {
                    let (g, i) = find(l, &tool).ok_or("that tool isn't on the toolbar")?;
                    let e = &mut l[g][i];
                    let next = on.unwrap_or(!e.on);
                    if !next && tool == ALWAYS_SHOWN {
                        return Err("Move can't be hidden: it's how you get back from every other tool".into());
                    }
                    e.on = next;
                    Ok(())
                })
            }
        ),
        spec!(
            "toolbar.move",
            "Move a toolbar tool",
            "{tool, group (index; the number of groups makes a new group at the end), index? (in the group; default: the end)} → the toolbar",
            false,
            |s, p| {
                let cmd = "toolbar.move";
                let tool = tool_param(cmd, p, "tool")?;
                let group = p.get("group").and_then(Value::as_u64).ok_or_else(|| bad(cmd, "`group` is required"))? as usize;
                let index = p.get("index").and_then(Value::as_u64).map(|i| i as usize);
                edit(s, cmd, |l| {
                    if group > l.len() {
                        return Err("no such group".into());
                    }
                    let (g, i) = find(l, &tool).ok_or("that tool isn't on the toolbar")?;
                    let e = l[g].remove(i);
                    // Removing the tool may have emptied (and so removed) its group: account for it.
                    let mut target = group;
                    if l[g].is_empty() {
                        l.remove(g);
                        if g < target {
                            target -= 1;
                        }
                    }
                    if target >= l.len() {
                        l.push(vec![e]);
                    } else {
                        let at = index.unwrap_or(l[target].len()).min(l[target].len());
                        l[target].insert(at, e);
                    }
                    Ok(())
                })
            }
        ),
        spec!(
            "toolbar.group",
            "Group a toolbar tool with another",
            "{tool, with} → the toolbar  (drop one tool onto another: it joins that tool's group, after it)",
            false,
            |s, p| {
                let cmd = "toolbar.group";
                let (tool, with) = (tool_param(cmd, p, "tool")?, tool_param(cmd, p, "with")?);
                edit(s, cmd, |l| {
                    if tool == with {
                        return Err("a tool can't be grouped with itself".into());
                    }
                    let (g, i) = find(l, &tool).ok_or("that tool isn't on the toolbar")?;
                    let e = l[g].remove(i);
                    l.retain(|g| !g.is_empty());
                    let (wg, wi) = find(l, &with).ok_or("the other tool isn't on the toolbar")?;
                    l[wg].insert(wi + 1, e);
                    Ok(())
                })
            }
        ),
        spec!(
            "toolbar.ungroup",
            "Take a tool out of its group",
            "{tool} → the toolbar  (it becomes a button of its own, after its group)",
            false,
            |s, p| {
                let cmd = "toolbar.ungroup";
                let tool = tool_param(cmd, p, "tool")?;
                edit(s, cmd, |l| {
                    let (g, i) = find(l, &tool).ok_or("that tool isn't on the toolbar")?;
                    if l[g].len() == 1 {
                        return Ok(());
                    }
                    let e = l[g].remove(i);
                    l.insert(g + 1, vec![e]);
                    Ok(())
                })
            }
        ),
        spec!("toolbar.reset", "Reset the toolbar", "{} → the toolbar  (the default tools, order and groups)", false, |s, _| {
            s.prefs.settings.toolbar = Vec::new();
            Ok(state(s))
        }),
    ]
}

#[cfg(test)]
#[path = "toolbar_tests.rs"]
mod tests;
