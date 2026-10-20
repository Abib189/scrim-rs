//! One member of `mcpServers` changed in the file's own text, so the rest keeps its formatting:
//! indentation, key order, spacing and the other servers byte for byte. A small scanner finds the
//! members' spans; the caller parses the result and checks it means what it should, and anything
//! unexpected here returns `None` (the file is then pretty-printed instead).

use serde_json::Value;

use super::Change;

/// A member of a JSON object: where its key starts, and its value's span.
struct Member {
    key: String,
    start: usize,
    value: (usize, usize),
}

/// An object: its members, and where its `{` and `}` are.
struct Object {
    open: usize,
    close: usize,
    members: Vec<Member>,
}

fn ws(b: &[u8], mut i: usize) -> usize {
    while b.get(i).is_some_and(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r')) {
        i += 1;
    }
    i
}

/// The end of the string starting at `i` (its opening quote), after its closing quote.
fn string_end(b: &[u8], i: usize) -> Option<usize> {
    if b.get(i) != Some(&b'"') {
        return None;
    }
    let mut j = i + 1;
    loop {
        match b.get(j)? {
            b'\\' => j += 2,
            b'"' => return Some(j + 1),
            _ => j += 1,
        }
    }
}

/// The end of the value starting at `i`.
fn value_end(b: &[u8], i: usize) -> Option<usize> {
    match b.get(i)? {
        b'"' => string_end(b, i),
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut j = i;
            loop {
                match b.get(j)? {
                    b'"' => {
                        j = string_end(b, j)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth = depth.checked_sub(1)?;
                        if depth == 0 {
                            return Some(j + 1);
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        }
        _ => {
            let mut j = i;
            while b.get(j).is_some_and(|c| !matches!(c, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')) {
                j += 1;
            }
            (j > i).then_some(j)
        }
    }
}

/// The object whose `{` is at `open`.
fn object(text: &str, open: usize) -> Option<Object> {
    let b = text.as_bytes();
    if b.get(open) != Some(&b'{') {
        return None;
    }
    let mut members = Vec::new();
    let mut i = ws(b, open + 1);
    if b.get(i) == Some(&b'}') {
        return Some(Object { open, close: i, members });
    }
    loop {
        let start = i;
        let key_end = string_end(b, i)?;
        let key: String = serde_json::from_str(text.get(start..key_end)?).ok()?;
        i = ws(b, key_end);
        if b.get(i) != Some(&b':') {
            return None;
        }
        let v0 = ws(b, i + 1);
        let v1 = value_end(b, v0)?;
        members.push(Member { key, start, value: (v0, v1) });
        i = ws(b, v1);
        match b.get(i)? {
            b',' => i = ws(b, i + 1),
            b'}' => return Some(Object { open, close: i, members }),
            _ => return None,
        }
    }
}

/// The whitespace a line starts with, for the line `at` is on.
fn indent_at(text: &str, at: usize) -> String {
    let line = text.get(..at).and_then(|t| t.rfind('\n')).map_or(0, |n| n + 1);
    text.get(line..at).map(|s| s.chars().take_while(|c| *c == ' ' || *c == '\t').collect()).unwrap_or_default()
}

/// The indent one level in: the text's own unit (its first indented line), else two spaces.
fn unit(text: &str) -> String {
    text.lines().map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').collect::<String>()).find(|s| !s.is_empty()).unwrap_or_else(|| "  ".into())
}

/// `v` pretty-printed with two spaces, an entry's keys in the order people write them (`command`,
/// `args`, `env`, then the rest).
pub fn ordered(v: &Value) -> Option<String> {
    fn go(v: &Value, depth: usize, out: &mut String) -> Option<()> {
        let pad = |d: usize| "  ".repeat(d);
        match v {
            Value::Object(m) if !m.is_empty() => {
                let rank = |k: &str| ["command", "args", "env"].iter().position(|x| *x == k).unwrap_or(3);
                let mut keys: Vec<&String> = m.keys().collect();
                keys.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
                out.push_str("{\n");
                for (i, k) in keys.iter().enumerate() {
                    out.push_str(&format!("{}{}: ", pad(depth + 1), serde_json::to_string(k).ok()?));
                    go(m.get(*k)?, depth + 1, out)?;
                    out.push_str(if i + 1 < keys.len() { ",\n" } else { "\n" });
                }
                out.push_str(&format!("{}}}", pad(depth)));
            }
            Value::Array(a) if !a.is_empty() => {
                out.push_str("[\n");
                for (i, x) in a.iter().enumerate() {
                    out.push_str(&pad(depth + 1));
                    go(x, depth + 1, out)?;
                    out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
                }
                out.push_str(&format!("{}]", pad(depth)));
            }
            _ => out.push_str(&serde_json::to_string(v).ok()?),
        }
        Some(())
    }
    let mut out = String::new();
    go(v, 0, &mut out)?;
    Some(out)
}

/// `v` pretty-printed with every line after the first indented by `indent`, in the text's `unit`.
fn pretty(v: &Value, indent: &str, unit: &str) -> Option<String> {
    let s = ordered(v)?;
    let lines: Vec<String> = s
        .lines()
        .enumerate()
        .map(|(n, l)| {
            let depth = l.chars().take_while(|c| *c == ' ').count() / 2;
            let body = l.trim_start();
            if n == 0 { body.to_string() } else { format!("{indent}{}{body}", unit.repeat(depth)) }
        })
        .collect();
    Some(lines.join("\n"))
}

/// `obj` with member `key: v` added at its end, in the style of its other members.
fn insert(text: &str, obj: &Object, key: &str, v: &Value) -> Option<String> {
    let unit = unit(text);
    let close_indent = indent_at(text, obj.close);
    let (indent, sep_newline) = match obj.members.first() {
        Some(m) => (indent_at(text, m.start), text.get(obj.open..m.start)?.contains('\n')),
        None => (format!("{close_indent}{unit}"), true),
    };
    let member = format!("{}: {}", serde_json::to_string(key).ok()?, pretty(v, &indent, &unit)?);
    match obj.members.last() {
        Some(last) => {
            let at = last.value.1;
            let sep = if sep_newline { format!(",\n{indent}") } else { ", ".into() };
            Some(format!("{}{sep}{member}{}", text.get(..at)?, text.get(at..)?))
        }
        None => Some(format!("{}\n{indent}{member}\n{close_indent}{}", text.get(..obj.open + 1)?, text.get(obj.close..)?)),
    }
}

/// `text` with `change` made to `mcpServers`, or `None` when the text isn't in a shape this
/// handles (the caller then pretty-prints the file).
pub fn apply(text: &str, change: &Change) -> Option<String> {
    let b = text.as_bytes();
    let top = object(text, ws(b, 0))?;
    let servers = top.members.iter().find(|m| m.key == "mcpServers");
    match (change, servers) {
        (Change::Set(k, v), None) => {
            let mut inner = serde_json::Map::new();
            inner.insert(k.clone(), v.clone());
            insert(text, &top, "mcpServers", &Value::Object(inner))
        }
        (Change::Remove(_), None) => Some(text.to_string()),
        (change, Some(s)) => {
            let obj = object(text, s.value.0)?;
            let key = match change {
                Change::Set(k, _) | Change::Remove(k) => k,
            };
            let at = obj.members.iter().position(|m| m.key == *key);
            match (change, at) {
                (Change::Set(_, v), Some(i)) => {
                    let m = obj.members.get(i)?;
                    let indent = indent_at(text, m.start);
                    Some(format!("{}{}{}", text.get(..m.value.0)?, pretty(v, &indent, &unit(text))?, text.get(m.value.1..)?))
                }
                (Change::Set(k, v), None) => insert(text, &obj, k, v),
                (Change::Remove(_), None) => Some(text.to_string()),
                (Change::Remove(_), Some(i)) => {
                    let m = obj.members.get(i)?;
                    let (cut0, cut1) = match (i.checked_sub(1).and_then(|p| obj.members.get(p)), obj.members.get(i + 1)) {
                        // After another member: from the end of the one before (its comma goes).
                        (Some(prev), _) => (prev.value.1, m.value.1),
                        // The first of several: up to the next one's key.
                        (None, Some(next)) => (m.start, next.start),
                        // The only one: the object becomes empty.
                        (None, None) => (obj.open + 1, obj.close),
                    };
                    Some(format!("{}{}", text.get(..cut0)?, text.get(cut1..)?))
                }
            }
        }
    }
}
