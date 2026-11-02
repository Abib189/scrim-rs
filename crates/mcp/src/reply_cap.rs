//! Every MCP reply fits what a client can read (Fix AI C-4, ported from upstream FigCraft b1f0eafb
//! and 1aa1d61b).
//!
//! MCP stdio is newline-delimited JSON, so one result is one message and has to fit the client's
//! read buffer whole: there is no streaming of part of one. The TypeScript SDK's stdio transport
//! defaults to 10 MiB and hosts rarely raise it; past that the client stops reading, the pipe
//! breaks and every later call in the session fails.
//!
//! - `inspect_document` answers at the deepest depth that fits, found by bisection.
//! - `get_node` does the same, then takes a window over the children when even depth 1 is too
//!   large (a frame with thousands of children), with the `offset` to ask for next.
//! - Both say so in a `truncated` object; a reply that fits is sent unchanged.
//! - Every other reply that would pass the limit is withheld whole with a `truncated` object
//!   giving its size and how to ask for less ([`backstop`]): JSON cut at a byte count isn't JSON,
//!   and a write that ran must not look like one that failed.

use serde_json::{Value, json};

use crate::backend::Backend;
use crate::tools::ToolResult;

/// What one reply may take on the wire by default: the usual 10 MiB client buffer less 2 MiB for
/// the JSON-RPC envelope.
pub(crate) const MAX_RESPONSE: usize = 8 << 20;

/// Room kept for the `truncated` object, which is added after the size is measured.
const MARKER_RESERVE: usize = 1 << 10;

/// Room kept for what the text block is wrapped in: the content object, `isError`, the JSON-RPC
/// envelope and the request id (any id up to about 900 bytes).
const ENVELOPE_RESERVE: usize = 1 << 10;

/// The limit a caller gave as `maxBytes` (default [`MAX_RESPONSE`]).
pub(crate) fn max_bytes(v: Option<f64>) -> Result<usize, String> {
    match v {
        Some(v) if v < 1024.0 => Err("`maxBytes` must be at least 1024".into()),
        Some(v) => Ok(v.min(usize::MAX as f64) as usize),
        None => Ok(MAX_RESPONSE),
    }
}

/// What the tree itself may take of `max` bytes.
fn budget(max: usize) -> usize {
    max.saturating_sub(ENVELOPE_RESERVE)
}

/// What a value's text block takes on the wire: pretty-printed, as [`ToolResult::json`] sends it,
/// and escaped inside the JSON-RPC message (escaping adds about a tenth).
fn wire_size(value: &Value) -> usize {
    let text = serde_json::to_string_pretty(value).unwrap_or_default();
    serde_json::to_string(&text).map_or(text.len(), |s| s.len())
}

fn inspect_at(b: &mut dyn Backend, depth: u64, all: bool) -> Result<(Value, usize), String> {
    let value = b.call("document.inspect", json!({"depth": depth, "allPages": all}))?;
    let wire = wire_size(&value);
    Ok((value, wire))
}

fn node_at(b: &mut dyn Backend, id: u64, depth: u64) -> Result<(Value, usize), String> {
    let value = b.call("node.get", json!({"id": id, "depth": depth}))?;
    let wire = wire_size(&value);
    Ok((value, wire))
}

/// The deepest depth in `lo..=hi` whose answer fits `budget` with the marker, by bisection: a
/// deeper tree holds every node of a shallower one, so the size doesn't fall as the depth grows.
/// `None`: not even `lo` fits (its answer comes back anyway, to be windowed or sent with the
/// marker).
fn deepest(lo: u64, hi: u64, budget: usize, mut at: impl FnMut(u64) -> Result<(Value, usize), String>) -> Result<(u64, Value, usize, bool), String> {
    let (mut lo, mut hi) = (lo, hi);
    let floor = lo;
    let mut best: Option<(u64, Value, usize)> = None;
    while lo <= hi {
        let mid = lo + (hi - lo) / 2;
        let (v, w) = at(mid)?;
        if w + MARKER_RESERVE <= budget {
            best = Some((mid, v, w));
            lo = mid + 1;
        } else if mid == floor {
            return Ok((mid, v, w, false));
        } else {
            hi = mid - 1;
        }
    }
    match best {
        Some((d, v, w)) => Ok((d, v, w, true)),
        None => at(floor).map(|(v, w)| (floor, v, w, false)),
    }
}

/// The document at `depth`, or at the deepest depth that fits `max` bytes with a `truncated`
/// object saying so. Depth 0 is the shallowest tree there is: when even that is over, it is sent
/// with the marker, since an error would leave the caller with nothing.
pub(crate) fn inspect(b: &mut dyn Backend, depth: u64, all: bool, max: usize) -> Result<Value, String> {
    let budget = budget(max);
    let (value, wire) = inspect_at(b, depth, all)?;
    if wire <= budget {
        return Ok(value);
    }
    let (used, mut value, used_wire) = if depth == 0 {
        (0, value, wire)
    } else {
        let (d, v, w, _) = deepest(0, depth - 1, budget, |d| inspect_at(b, d, all))?;
        (d, v, w)
    };
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "truncated".into(),
            json!({
                "requestedDepth": depth,
                "depth": used,
                "requestedDepthBytes": wire,
                "bytes": used_wire,
                "maxBytes": max,
                "reason": format!("the tree at depth {depth} would be {wire} bytes on the wire, over the {max}-byte limit a client can read in one message; it is reported at depth {used} instead"),
                "hint": "get_node({id, depth}) reads one subtree at a time, within the same limit: a layer with more children than fit answers with a window of them and the offset to ask for next",
            }),
        );
    }
    Ok(value)
}

/// One layer's subtree, kept inside `max` bytes: cut by depth first, as the document's tree is,
/// then by a window over the children from `offset` (the width of a wide layer is at depth 1, so
/// no depth makes it smaller). `None`: no such layer.
pub(crate) fn node(b: &mut dyn Backend, id: u64, depth: u64, offset: usize, max: usize) -> Result<Option<Value>, String> {
    let budget = budget(max);
    let (value, wire) = node_at(b, id, depth)?;
    if value.is_null() {
        return Ok(None);
    }
    if wire <= budget && offset == 0 {
        return Ok(Some(value));
    }
    let (mut used, mut value) = (depth, value);
    if depth > 1 && offset == 0 {
        let (d, v, w, fits) = deepest(1, depth - 1, budget, |d| node_at(b, id, d))?;
        if fits {
            return Ok(Some(with_marker(v, depth, d, wire, w, max, None)));
        }
        (used, value) = (d, v);
    }
    // Still over, or a window was asked for: the children from `offset` that fit.
    let total = value.get("children").and_then(Value::as_array).map_or(0, Vec::len);
    let cut = children_window(&mut value, offset, budget.saturating_sub(MARKER_RESERVE));
    let shown = value.get("children").and_then(Value::as_array).map_or(0, Vec::len);
    let next = (offset + shown < total).then_some(offset + shown);
    if !cut && used == depth {
        // An offset of zero that fitted whole after all.
        return Ok(Some(value));
    }
    let used_wire = wire_size(&value);
    let window = json!({"from": offset, "shown": shown, "total": total, "nextOffset": next});
    Ok(Some(with_marker(value, depth, used, wire, used_wire, max, Some(window))))
}

/// Keep the children from `offset` that fit in `budget`; whether any were left out. Bisection
/// keeps a node with thousands of children from being serialised once per child.
fn children_window(value: &mut Value, offset: usize, budget: usize) -> bool {
    let Some(all) = value.get("children").and_then(Value::as_array).cloned() else { return false };
    let rest: Vec<Value> = all.iter().skip(offset).cloned().collect();
    let fits = |value: &mut Value, n: usize| {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("children".into(), Value::Array(rest.iter().take(n).cloned().collect()));
        }
        wire_size(value) <= budget
    };
    if fits(value, rest.len()) {
        return rest.len() != all.len();
    }
    let (mut lo, mut hi) = (0usize, rest.len());
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if fits(value, mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    fits(value, lo);
    true
}

/// `value` with the `truncated` object saying how it was cut; `window`: the children shown, when
/// they were cut too.
fn with_marker(mut value: Value, asked: u64, used: u64, asked_wire: usize, used_wire: usize, max: usize, window: Option<Value>) -> Value {
    let mut marker = json!({"requestedDepth": asked, "depth": used, "requestedDepthBytes": asked_wire, "bytes": used_wire, "maxBytes": max});
    let over =
        format!("the subtree at depth {asked} would be {asked_wire} bytes on the wire, over the {max}-byte limit a client can read in one message");
    match window {
        Some(w) => {
            let from = w.get("from").and_then(Value::as_u64).unwrap_or(0);
            marker["reason"] = json!(format!("{over}; its children are reported from {from} instead"));
            marker["hint"] = json!("ask again with the `offset` in `children.nextOffset` for the rest, or get_node({id, depth}) on one child");
            marker["children"] = w;
        }
        None => {
            marker["reason"] = json!(format!("{over}; it is reported at depth {used} instead"));
            marker["hint"] = json!("get_node({id, depth}) on a child reads deeper, one subtree at a time");
        }
    }
    if let Some(obj) = value.as_object_mut() {
        obj.insert("truncated".into(), marker);
    }
    value
}

/// Tool `name`'s reply `r`, or, when it would pass `max` bytes on the wire, a reply in its place
/// that withholds it and says how to ask for less. `isError` is kept: a write that ran still ran.
pub(crate) fn backstop(name: &str, r: ToolResult, max: usize) -> ToolResult {
    let bytes = serde_json::to_string(&r.to_value()).map_or(0, |s| s.len());
    if bytes <= budget(max) {
        return r;
    }
    let hint = match name {
        "export" => "pass `path` to write the file there instead, or a smaller `scale` or `size`",
        "screenshot" => "pass a smaller `maxSize`, or `ids` for fewer layers",
        "run_command" => "an export command writes to a `path`; read the document with inspect_document or get_node, which keep to the limit",
        "batch" => "split the batch into smaller ones",
        "lint_design" | "analyze_design" => "pass `id` to cover one subtree at a time",
        "get_changes" => "pass a later `since`",
        "open_file" => "the file is open: read it with inspect_document or get_node",
        _ => "ask for less at a time",
    };
    let what = if r.is_error { "its error message was withheld" } else { "the call ran; only its reply was withheld" };
    let v = json!({"truncated": {
        "bytes": bytes,
        "maxBytes": max,
        "reason": format!("the reply would be {bytes} bytes on the wire, over the {max}-byte limit a client can read in one message: {what}"),
        "hint": hint,
    }});
    let mut out = ToolResult::json(&v);
    out.is_error = r.is_error;
    out
}
