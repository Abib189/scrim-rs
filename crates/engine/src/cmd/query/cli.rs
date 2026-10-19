//! The CLI's `query` and `find` (spec §12), with no I/O: the argument rules, the human and JSON
//! output, and the exit codes. The caller loads the file and runs the query (`run`).
//!
//! Exit codes: 0 done; 1 an invalid query or another tool error; 2 usage; 3 the file.

use serde::Serialize;

use super::{QueryOut, Vocab};

/// What the command printed and how it exited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliOutcome {
    pub exit: i32,
    pub stdout: String,
    pub stderr: String,
}

/// A failure, with its machine-readable code: `usage`, `invalid_query`, `tool_error`, or one of
/// the file codes (`not_found`, `not_a_file`, `too_large`, `unreadable`, `invalid_document`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CliFail {
    pub code: String,
    pub message: String,
}

impl CliFail {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }
    fn exit(&self) -> i32 {
        match self.code.as_str() {
            "usage" => 2,
            "invalid_query" | "tool_error" => 1,
            _ => 3,
        }
    }
}

/// JavaScript's `Number(raw)` for the forms a person types: decimal (`5`, `5e0`, ` 5 `, `+5`,
/// `.5`), hex, octal and binary (`0x10`); the empty string is 0. Anything else is NaN.
fn js_number_parse(raw: &str) -> f64 {
    let s = raw.trim();
    if s.is_empty() {
        return 0.0;
    }
    for (p, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(d) = s.strip_prefix(p) {
            return u64::from_str_radix(d, radix).map_or(f64::NAN, |v| v as f64);
        }
    }
    match s {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // Rust also reads "inf" and "nan"; JavaScript doesn't.
    if s.bytes().any(|b| b.is_ascii_alphabetic() && b != b'e' && b != b'E') {
        return f64::NAN;
    }
    s.parse().unwrap_or(f64::NAN)
}

/// Run `query <file> '<query>' [--limit N] [--json]` or `find <file> [--type T] [--name text]
/// [--limit N] [--json]`. `args` follow the command name. `prog` prefixes the messages.
/// `run(file, query, limit)` loads the file and runs the query.
pub fn run_cli(
    prog: &str,
    vocab: &Vocab,
    cmd: &str,
    args: &[String],
    run: impl FnOnce(&str, &str, Option<f64>) -> Result<QueryOut, CliFail>,
) -> CliOutcome {
    let json = args.iter().any(|a| a == "--json");
    match parse_and_run(prog, vocab, cmd, args, run) {
        Ok(out) if json => CliOutcome { exit: 0, stdout: format!("{}\n", out.text()), stderr: String::new() },
        Ok(out) => CliOutcome { exit: 0, stdout: human(&out), stderr: String::new() },
        Err(f) if json => {
            #[derive(Serialize)]
            struct Wrapped<'a> {
                error: &'a CliFail,
            }
            let text = serde_json::to_string_pretty(&Wrapped { error: &f }).unwrap_or_default();
            CliOutcome { exit: f.exit(), stdout: format!("{text}\n"), stderr: String::new() }
        }
        Err(f) => CliOutcome { exit: f.exit(), stdout: String::new(), stderr: format!("{prog}: {}\n", f.message) },
    }
}

fn parse_and_run(
    prog: &str,
    vocab: &Vocab,
    cmd: &str,
    args: &[String],
    run: impl FnOnce(&str, &str, Option<f64>) -> Result<QueryOut, CliFail>,
) -> Result<QueryOut, CliFail> {
    let usage = |m: String| CliFail::new("usage", m);
    let takes: &[&str] = if cmd == "find" { &["type", "name", "limit"] } else { &["limit"] };
    let mut flags: Vec<(String, String)> = Vec::new();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        let Some(flag) = a.strip_prefix("--") else {
            positional.push(a.clone());
            continue;
        };
        let (name, inline) = match flag.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (flag, None),
        };
        if name == "json" {
            continue;
        }
        if !takes.contains(&name) {
            let list: Vec<String> = takes.iter().map(|t| format!("--{t}")).chain(["--json".to_string()]).collect();
            return Err(usage(format!("{cmd} has no --{name} (it takes {})", list.join(", "))));
        }
        let value = match inline {
            Some(v) => v,
            None if i < args.len() => {
                i += 1;
                args[i - 1].clone()
            }
            None => return Err(usage(format!("--{name} needs a value"))),
        };
        flags.push((name.to_string(), value));
    }
    let flag = |n: &str| flags.iter().rev().find(|(k, _)| k == n).map(|(_, v)| v.as_str());
    let (file, query) = if cmd == "find" {
        let [file] = positional.as_slice() else { return Err(usage(format!("usage: {prog} find <file> [--type T] [--name text]"))) };
        if flag("type").is_none() && flag("name").is_none() {
            return Err(usage("find needs --type and/or --name".into()));
        }
        let test = match flag("type") {
            None => "*".to_string(),
            Some(t) if vocab.tests.contains(&t.to_uppercase().as_str()) => t.to_uppercase(),
            Some(_) => return Err(usage(format!("--type is one of {}", vocab.tests.join(", ")))),
        };
        let pred = match flag("name") {
            None => String::new(),
            Some(n) if n.contains('"') && n.contains('\'') => return Err(usage("--name can hold single or double quotes, not both".into())),
            Some(n) if n.contains('"') => format!("[contains(@name, '{n}')]"),
            Some(n) => format!("[contains(@name, \"{n}\")]"),
        };
        (file.clone(), format!("//{test}{pred}"))
    } else {
        let [file, query] = positional.as_slice() else { return Err(usage(format!("usage: {prog} query <file> '<query>'"))) };
        (file.clone(), query.clone())
    };
    let limit = match flag("limit") {
        None => None,
        Some(raw) => {
            let n = js_number_parse(raw);
            if n.fract() != 0.0 || !(1.0..=1000.0).contains(&n) {
                return Err(usage("--limit is a whole number from 1 to 1000".into()));
            }
            Some(n)
        }
    };
    run(&file, &query, limit)
}

/// `6 matches` then one line per result: `  TEXT      Logo  (logo)`.
fn human(out: &QueryOut) -> String {
    let mut s = if out.truncated {
        format!("{} of {} matches (--limit for more)\n", out.returned, out.total)
    } else if out.total == 1 {
        "1 match\n".to_string()
    } else {
        format!("{} matches\n", out.total)
    };
    for h in &out.results {
        let id = h.id.as_str().map_or_else(|| h.id.to_string(), str::to_string);
        s += &format!("  {:<9} {}  ({id})\n", h.element, h.name);
    }
    s
}
