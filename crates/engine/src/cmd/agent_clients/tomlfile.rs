//! A client's `config.toml` (Codex, Grok Build) read and changed with `toml_edit` (Stage J):
//! comments, ordering and formatting of everything else are kept. Only `container[..]` ▸ `key`
//! changes: set as its own table (`[mcp_servers.scrim]`), or removed.

use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item, Table};

/// `text` parsed: `None` for a missing or blank file.
pub fn parse(text: Option<&str>) -> Result<Option<DocumentMut>, String> {
    let Some(t) = text.filter(|t| !t.trim().is_empty()) else { return Ok(None) };
    t.parse::<DocumentMut>().map(Some).map_err(|e| format!("it isn't valid TOML ({})", e.message()))
}

/// A TOML value as JSON (for comparing an entry with what Scrim would write).
fn json(item: &Item) -> Value {
    match item {
        Item::Value(v) => value_json(v),
        Item::Table(t) => Value::Object(t.iter().map(|(k, v)| (k.to_string(), json(v))).collect()),
        Item::ArrayOfTables(a) => Value::Array(a.iter().map(|t| Value::Object(t.iter().map(|(k, v)| (k.to_string(), json(v))).collect())).collect()),
        Item::None => Value::Null,
    }
}

fn value_json(v: &toml_edit::Value) -> Value {
    match v {
        toml_edit::Value::String(s) => Value::String(s.value().clone()),
        toml_edit::Value::Integer(i) => Value::from(*i.value()),
        toml_edit::Value::Float(f) => serde_json::Number::from_f64(*f.value()).map_or(Value::Null, Value::Number),
        toml_edit::Value::Boolean(b) => Value::Bool(*b.value()),
        toml_edit::Value::Datetime(d) => Value::String(d.value().to_string()),
        toml_edit::Value::Array(a) => Value::Array(a.iter().map(value_json).collect()),
        toml_edit::Value::InlineTable(t) => Value::Object(t.iter().map(|(k, v)| (k.to_string(), value_json(v))).collect()),
    }
}

/// The servers table at `container` as JSON members (`None` when it's missing).
pub fn servers(doc: &DocumentMut, container: &[String]) -> Result<Option<Map<String, Value>>, String> {
    let mut item: &Item = doc.as_item();
    for seg in container {
        match item.get(seg.as_str()) {
            Some(i) => item = i,
            None => return Ok(None),
        }
    }
    match json(item) {
        Value::Object(m) => Ok(Some(m)),
        _ => Err(format!("its {} isn't a table", container.join("."))),
    }
}

/// A JSON entry as a TOML value (strings, numbers, booleans, arrays, and inline tables for nested
/// objects).
fn toml_value(v: &Value) -> Option<toml_edit::Value> {
    Some(match v {
        Value::String(s) => s.as_str().into(),
        Value::Bool(b) => (*b).into(),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i.into(),
            None => n.as_f64()?.into(),
        },
        Value::Array(a) => toml_edit::Value::Array(a.iter().map(toml_value).collect::<Option<toml_edit::Array>>()?),
        Value::Object(o) => {
            let mut t = toml_edit::InlineTable::new();
            for (k, v) in o {
                t.insert(k, toml_value(v)?);
            }
            toml_edit::Value::InlineTable(t)
        }
        Value::Null => return None,
    })
}

/// `text` (or a new file) with `container[..]` ▸ `key` set to `entry` (`Some`) or removed (`None`).
pub fn change(text: Option<&str>, container: &[String], key: &str, entry: Option<&Value>) -> Result<String, String> {
    let mut doc = parse(text)?.unwrap_or_default();
    let mut table: &mut Table = doc.as_table_mut();
    for seg in container {
        if !table.contains_key(seg) {
            if entry.is_none() {
                return Ok(doc.to_string());
            }
            let mut t = Table::new();
            // `[mcp_servers.scrim]`, not an empty `[mcp_servers]` header above it.
            t.set_implicit(true);
            table.insert(seg, Item::Table(t));
        }
        table = table.get_mut(seg).and_then(Item::as_table_mut).ok_or_else(|| format!("its {seg} isn't a table"))?;
    }
    match entry {
        Some(Value::Object(fields)) => {
            let mut t = Table::new();
            for (k, v) in fields {
                t.insert(k, Item::Value(toml_value(v).ok_or("the entry can't be written as TOML")?));
            }
            table.insert(key, Item::Table(t));
        }
        Some(_) => return Err("the entry isn't a table".into()),
        None => {
            table.remove(key);
        }
    }
    Ok(doc.to_string())
}
