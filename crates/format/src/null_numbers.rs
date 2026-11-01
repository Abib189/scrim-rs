//! Files saved with `null` where a number belongs still open (Fix AI LW-2). JSON has no NaN or
//! infinity, so `serde_json` wrote them as `null`, and reading the file back failed on the first
//! one: the whole file wouldn't open. Saving now writes 0 instead ([`crate::finite`]); a file
//! saved before opens like this:
//!
//! - each layer (and the document's other parts) that a `null` stops from reading is read again
//!   with that `null` left out, so the property takes its default, or 0 for a number the file
//!   must have (a list item, or a field with no default);
//! - a `null` that reads fine (an optional value that is absent) stays as it is: every one is put
//!   back on its own and kept wherever the layer still reads.
//!
//! The repair happens in memory, once per damaged layer: opening never writes the file.

use serde::Deserialize;
use serde_json::{Map, Value};

use figcraft_doc::{Document, Node};

/// Most nulls one file may have reset.
pub const MAX_REPAIRS: usize = 10_000;
/// Most nulls in one layer each tried on its own (see [`repair_with`]).
const MAX_TRIED: usize = 256;

/// Whether `e` says a value read as `null` must be something else.
pub(crate) fn is_null_error(e: &serde_json::Error) -> bool {
    e.to_string().starts_with("invalid type: null")
}

/// Repair `doc`, a whole `document.json`, so that its layers and other parts read; the number of
/// nulls reset (0 when none stopped it from reading). Parts that fail for another reason are left
/// as they are, for the caller's reading to report.
pub(crate) fn repair(doc: &mut Value) -> Result<usize, String> {
    let mut count = 0;
    if let Some(nodes) = doc.get_mut("nodes").and_then(Value::as_object_mut) {
        for node in nodes.values_mut() {
            count += repair_with(node, |v| Node::deserialize(v).map(drop));
            if count > MAX_REPAIRS {
                return Err(too_many());
            }
        }
    }
    // The document's other parts (styles, variables, comments…), read without its layers.
    let nodes = doc.get_mut("nodes").map(|n| std::mem::replace(n, Value::Object(Map::new())));
    count += repair_with(doc, |v| Document::deserialize(v).map(drop));
    if let (Some(slot), Some(nodes)) = (doc.get_mut("nodes"), nodes) {
        *slot = nodes;
    }
    if count > MAX_REPAIRS { Err(too_many()) } else { Ok(count) }
}

/// What a file whose `count` numbers were reset notes, once.
pub fn note(count: usize) -> String {
    if count == 1 {
        "1 number saved as null (not a number, by an earlier build) was reset to its default; the file itself wasn't changed".into()
    } else {
        format!("{count} numbers saved as null (not a number, by an earlier build) were reset to their defaults; the file itself wasn't changed")
    }
}

/// Repair one layer, `node` (as JSON), so it reads; the number of nulls reset. For a layer an
/// import made with a NaN or infinite number: written as JSON, those are `null`.
pub fn repair_node(node: &mut Value) -> usize {
    repair_with(node, |v| Node::deserialize(v).map(drop))
}

fn too_many() -> String {
    format!("more than {MAX_REPAIRS} of its numbers were saved as null (not a number)")
}

/// One step into a JSON value.
#[derive(Clone, Debug, PartialEq)]
enum Step {
    Key(String),
    Index(usize),
}

/// What became of one `null`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Fix {
    /// The member is left out: its property takes its default.
    Dropped,
    /// It reads as 0.
    Zero,
    /// It is `null` again: it read fine.
    Kept,
}

/// Repair `v` so `reads` accepts it, changing only the nulls that stop it; how many changed.
fn repair_with(v: &mut Value, reads: impl Fn(&Value) -> Result<(), serde_json::Error>) -> usize {
    match reads(v) {
        Err(e) if is_null_error(&e) => {}
        _ => return 0,
    }
    let nulls = null_paths(v);
    // Every null fixed: members left out, list items 0.
    let mut fixes: Vec<Fix> = nulls.iter().map(|p| if matches!(p.last(), Some(Step::Key(_))) { Fix::Dropped } else { Fix::Zero }).collect();
    let mut fixed = with_fixes(v, &nulls, &fixes);
    // A field the layer must have: 0 where leaving it out is refused.
    loop {
        match reads(&fixed) {
            Ok(()) => break,
            Err(e) => {
                let Some(field) = missing_field(&e) else { return 0 };
                let Some(i) = nulls.iter().zip(&fixes).position(|(p, f)| *f == Fix::Dropped && p.last() == Some(&Step::Key(field.clone()))) else {
                    return 0;
                };
                if let Some(f) = fixes.get_mut(i) {
                    *f = Fix::Zero;
                }
                fixed = with_fixes(v, &nulls, &fixes);
            }
        }
    }
    // The nulls that read fine stay: each put back on its own, and kept if it still reads. Past
    // [`MAX_TRIED`] nulls in one part, only left-out members are tried (list items are numbers).
    let all = nulls.len() <= MAX_TRIED;
    for i in 0..fixes.len() {
        if !all && fixes.get(i) != Some(&Fix::Dropped) {
            continue;
        }
        let was = fixes.get(i).copied();
        if let Some(f) = fixes.get_mut(i) {
            *f = Fix::Kept;
        }
        let trial = with_fixes(v, &nulls, &fixes);
        if reads(&trial).is_ok() {
            fixed = trial;
        } else if let (Some(f), Some(was)) = (fixes.get_mut(i), was) {
            *f = was;
        }
    }
    *v = fixed;
    fixes.iter().filter(|f| **f != Fix::Kept).count()
}

/// `v` with each null at `paths` treated as `fixes` says.
fn with_fixes(v: &Value, paths: &[Vec<Step>], fixes: &[Fix]) -> Value {
    let mut out = v.clone();
    for (path, fix) in paths.iter().zip(fixes) {
        let Some((last, parent)) = path.split_last() else { continue };
        let Some(holder) = at_mut(&mut out, parent) else { continue };
        match (fix, last, holder) {
            (Fix::Dropped, Step::Key(k), Value::Object(o)) => {
                o.remove(k);
            }
            (Fix::Zero, Step::Key(k), Value::Object(o)) => {
                o.insert(k.clone(), Value::from(0));
            }
            (Fix::Zero, Step::Index(i), Value::Array(a)) => {
                if let Some(item) = a.get_mut(*i) {
                    *item = Value::from(0);
                }
            }
            _ => {}
        }
    }
    out
}

fn at_mut<'a>(v: &'a mut Value, path: &[Step]) -> Option<&'a mut Value> {
    path.iter().try_fold(v, |v, step| match (step, v) {
        (Step::Key(k), Value::Object(o)) => o.get_mut(k),
        (Step::Index(i), Value::Array(a)) => a.get_mut(*i),
        _ => None,
    })
}

/// Where `v` holds `null`, in document order.
fn null_paths(v: &Value) -> Vec<Vec<Step>> {
    fn walk(v: &Value, path: &mut Vec<Step>, out: &mut Vec<Vec<Step>>) {
        match v {
            Value::Null => out.push(path.clone()),
            Value::Object(o) => {
                for (k, c) in o {
                    path.push(Step::Key(k.clone()));
                    walk(c, path, out);
                    path.pop();
                }
            }
            Value::Array(a) => {
                for (i, c) in a.iter().enumerate() {
                    path.push(Step::Index(i));
                    walk(c, path, out);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(v, &mut Vec::new(), &mut out);
    out
}

/// The field a "missing field `x`" error names.
fn missing_field(e: &serde_json::Error) -> Option<String> {
    let m = e.to_string();
    let rest = m.strip_prefix("missing field `")?;
    Some(rest.split('`').next()?.to_string())
}
