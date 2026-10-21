//! One member of an object at a key path changed in a JSON or JSONC file's own text (Stage J):
//! comments, order, spacing and every other member stay byte for byte. Comments (`//`, `/* */`) and
//! trailing commas are allowed, as VS Code, Zed, Cursor and OpenCode allow them.
//!
//! A small scanner finds the members' spans. The caller parses the result ([`parse`]) and checks it
//! means exactly what it should; anything this doesn't handle returns `None`, and then a file with
//! comments is not written at all (one without may be pretty-printed instead).

use serde_json::Value;

/// `text` without its comments and trailing commas (strings left alone), for parsing.
pub fn strip(text: &str) -> String {
    let b = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (b[i], b.get(i + 1)) {
            (b'"', _) => {
                let end = string_end(b, i).unwrap_or(b.len());
                out.extend_from_slice(&b[i..end]);
                i = end;
            }
            (b'/', Some(b'/')) => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            (b'/', Some(b'*')) => {
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    // Keep line breaks, so error positions still match lines.
                    if b[i] == b'\n' {
                        out.push(b'\n');
                    }
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            (b',', _) => {
                // A trailing comma: the next thing that isn't space is `}` or `]`.
                let mut j = i + 1;
                while b.get(j).is_some_and(u8::is_ascii_whitespace) {
                    j += 1;
                }
                if !matches!(b.get(j), Some(b'}' | b']')) {
                    out.push(b',');
                }
                i += 1;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Whether `text` has a comment outside its strings.
pub fn has_comments(text: &str) -> bool {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match (b[i], b.get(i + 1)) {
            (b'"', _) => i = string_end(b, i).unwrap_or(b.len()),
            (b'/', Some(b'/' | b'*')) => return true,
            _ => i += 1,
        }
    }
    false
}

/// `text` parsed as JSON with comments: `None` for a missing or blank file; an error for anything
/// that isn't a JSON object.
pub fn parse(text: Option<&str>) -> Result<Option<serde_json::Map<String, Value>>, String> {
    let Some(t) = text.filter(|t| !t.trim().is_empty()) else { return Ok(None) };
    match serde_json::from_str::<Value>(&strip(t)) {
        Ok(Value::Object(m)) => Ok(Some(m)),
        Ok(_) => Err("the file isn't a JSON object".into()),
        Err(e) => Err(format!("it isn't valid JSON ({e})")),
    }
}

/// Past whitespace and comments.
fn skip(b: &[u8], mut i: usize) -> usize {
    loop {
        while b.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        match (b.get(i), b.get(i + 1)) {
            (Some(b'/'), Some(b'/')) => {
                while b.get(i).is_some_and(|c| *c != b'\n') {
                    i += 1;
                }
            }
            (Some(b'/'), Some(b'*')) => {
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            _ => return i,
        }
    }
}

/// The index after the string starting at `i` (its quote).
fn string_end(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return Some(j + 1),
            _ => j += 1,
        }
    }
    None
}

/// The index after the value starting at `i`.
fn value_end(b: &[u8], i: usize) -> Option<usize> {
    match b.get(i)? {
        b'"' => string_end(b, i),
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut j = i;
            while j < b.len() {
                match b[j] {
                    b'"' => {
                        j = string_end(b, j)?;
                        continue;
                    }
                    b'/' if matches!(b.get(j + 1), Some(b'/' | b'*')) => {
                        j = skip(b, j);
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
            None
        }
        _ => {
            let mut j = i;
            while j < b.len() && !matches!(b[j], b',' | b'}' | b']' | b'/') && !b[j].is_ascii_whitespace() {
                j += 1;
            }
            (j > i).then_some(j)
        }
    }
}

struct Member {
    key: String,
    /// Where its key's quote is.
    start: usize,
    value: (usize, usize),
}

struct Object {
    open: usize,
    close: usize,
    members: Vec<Member>,
}

/// The object whose `{` is at `open`.
fn object(text: &str, open: usize) -> Option<Object> {
    let b = text.as_bytes();
    if b.get(open) != Some(&b'{') {
        return None;
    }
    let mut members = Vec::new();
    let mut i = skip(b, open + 1);
    loop {
        match b.get(i)? {
            b'}' => return Some(Object { open, close: i, members }),
            b'"' => {
                let kend = string_end(b, i)?;
                let key: String = serde_json::from_str(text.get(i..kend)?).ok()?;
                let colon = skip(b, kend);
                if b.get(colon) != Some(&b':') {
                    return None;
                }
                let vstart = skip(b, colon + 1);
                let vend = value_end(b, vstart)?;
                members.push(Member { key, start: i, value: (vstart, vend) });
                i = skip(b, vend);
                if b.get(i) == Some(&b',') {
                    i = skip(b, i + 1);
                }
            }
            _ => return None,
        }
    }
}

/// The text from the start of `at`'s line up to `at` when it is only spaces or tabs.
fn indent_at(text: &str, at: usize) -> String {
    let line = text.get(..at).and_then(|s| s.rsplit('\n').next()).unwrap_or_default();
    if line.chars().all(|c| c == ' ' || c == '\t') { line.to_string() } else { String::new() }
}

/// The file's indentation step: a tab when it indents with tabs, else two spaces (or four when it
/// uses four).
fn unit(text: &str) -> String {
    let first = text.lines().map(|l| l.len() - l.trim_start().len()).find(|n| *n > 0);
    if text.lines().any(|l| l.starts_with('\t')) {
        "\t".into()
    } else if first == Some(4) {
        "    ".into()
    } else {
        "  ".into()
    }
}

/// `v` pretty-printed with `unit`, every line after the first indented by `indent`.
fn pretty(v: &Value, indent: &str, unit: &str) -> Option<String> {
    use serde::Serialize;
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(unit.as_bytes());
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    v.serialize(&mut ser).ok()?;
    let s = String::from_utf8(buf).ok()?;
    Some(s.lines().enumerate().map(|(i, l)| if i == 0 { l.to_string() } else { format!("{indent}{l}") }).collect::<Vec<_>>().join("\n"))
}

/// `obj` with member `key` = `v` added at its end.
fn insert(text: &str, obj: &Object, key: &str, v: &Value) -> Option<String> {
    let b = text.as_bytes();
    let unit = unit(text);
    let outer = indent_at(text, obj.close);
    let inner = obj.members.first().map(|m| indent_at(text, m.start)).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("{outer}{unit}"));
    let member = format!("{}: {}", serde_json::to_string(key).ok()?, pretty(v, &inner, &unit)?);
    match obj.members.last() {
        Some(last) => {
            // After the last member, or after its trailing comma when it has one.
            let after = skip(b, last.value.1);
            let (at, lead) = if b.get(after) == Some(&b',') { (after + 1, String::new()) } else { (last.value.1, ",".to_string()) };
            let trailing = if lead.is_empty() { "," } else { "" };
            Some(format!("{}{lead}\n{inner}{member}{trailing}{}", text.get(..at)?, text.get(at..)?))
        }
        None => {
            let body = text.get(obj.open + 1..obj.close)?;
            // Comments inside an empty object stay, before the new member.
            let kept = if body.trim().is_empty() { String::new() } else { body.trim_end().to_string() };
            Some(format!("{}{kept}\n{inner}{member}\n{outer}{}", text.get(..=obj.open)?, text.get(obj.close..)?))
        }
    }
}

/// `text` with `path[..]` ▸ `key` set to `v`: an existing member's value replaced in place, a
/// missing one (or a missing container on the path) added at the end of its object.
pub fn set(text: &str, path: &[String], key: &str, v: &Value) -> Option<String> {
    let b = text.as_bytes();
    let mut obj = object(text, skip(b, 0))?;
    for (n, seg) in path.iter().enumerate() {
        match obj.members.iter().find(|m| m.key == *seg) {
            Some(m) => obj = object(text, m.value.0)?,
            None => {
                // The rest of the path doesn't exist: add it, holding the member.
                let mut value = serde_json::Map::new();
                value.insert(key.to_string(), v.clone());
                let nested = path[n + 1..].iter().rev().fold(Value::Object(value), |inner, k| {
                    let mut m = serde_json::Map::new();
                    m.insert(k.clone(), inner);
                    Value::Object(m)
                });
                return insert(text, &obj, seg, &nested);
            }
        }
    }
    match obj.members.iter().find(|m| m.key == key) {
        Some(m) => {
            let indent = indent_at(text, m.start);
            Some(format!("{}{}{}", text.get(..m.value.0)?, pretty(v, &indent, &unit(text))?, text.get(m.value.1..)?))
        }
        None => insert(text, &obj, key, v),
    }
}

/// `text` without `path[..]` ▸ `key` (unchanged when it isn't there).
pub fn remove(text: &str, path: &[String], key: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut obj = object(text, skip(b, 0))?;
    for seg in path {
        match obj.members.iter().find(|m| m.key == *seg) {
            Some(m) => obj = object(text, m.value.0)?,
            None => return Some(text.to_string()),
        }
    }
    let Some(i) = obj.members.iter().position(|m| m.key == key) else { return Some(text.to_string()) };
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
