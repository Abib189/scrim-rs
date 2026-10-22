//! No personal data in the evidence (R1, `docs/design/pii-scan.md`). Every tracked text file under
//! `docs/` and `scripts/` is scanned for:
//! - **home folders:** `/Users/<name>`, `/home/<name>`, `C:\Users\<name>`, and the same folder as a
//!   tool encodes it in a folder name (`-Users-<name>-`);
//! - **device names:** `<name>.local`, and this machine's own login and host name, read when the
//!   scan runs (never written down here);
//! - **email addresses** (the reserved example domains are fine);
//! - **bearer tokens and API keys;**
//! - **session ids** (a JSON `"session"` value, an `Mcp-Session-Id`, a UUID);
//! - **IP addresses** other than 127.0.0.1.
//!
//! Placeholders (`/Users/<user>`, `<device>`, `<token>`) never match. Deliberate values that do are
//! listed in `xtask/pii-allow.txt`, each with its reason. Images aren't read here: the captures are
//! checked by reading their text (`scripts/ocr-captures.swift`), and a document's own content can't
//! be told from a pattern (`docs/design/pii-scan.md`).

use std::path::Path;
use std::process::Command;

use regex::Regex;

/// The allow-list, next to this task.
const ALLOW: &str = "xtask/pii-allow.txt";

/// Generic login names that are words too ("the test runner"), never reported as this machine's.
const GENERIC_NAMES: &[&str] = &["root", "runner", "user", "users", "admin", "ubuntu", "build", "builder", "vscode", "docker", "ci"];

/// One pattern: what it finds, and how.
struct Rule {
    what: &'static str,
    re: Regex,
}

/// Something that looks personal, and where.
#[derive(Debug, PartialEq)]
pub struct Finding {
    pub what: &'static str,
    pub text: String,
    pub line: usize,
}

/// The patterns, and this machine's own names (`local`: its login and host name).
pub struct Scanner {
    rules: Vec<Rule>,
    local: Vec<String>,
    allow: Vec<String>,
}

impl Scanner {
    /// The scanner, with `allow` (the allow-list's text) and this machine's names `local`.
    pub fn new(allow: &str, local: &[String]) -> Option<Self> {
        let rule = |what, pattern: &str| Regex::new(pattern).ok().map(|re| Rule { what, re });
        let rules = [
            rule("home folder", r"(?:^|[^A-Za-z0-9._/~>-])((?:/Users|/home)/[A-Za-z0-9._-]+)"),
            rule("home folder", r"(?i)\b[A-Z]:\\Users\\[A-Za-z0-9._-]+"),
            rule("home folder (encoded)", r"-Users-[A-Za-z0-9._]+-"),
            rule("device name", r"(?i)\b[A-Za-z0-9-]+\.local(?:[^A-Za-z0-9.]|$)"),
            rule("email address", r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}"),
            rule("bearer token", r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{12,}"),
            rule(
                "API key",
                r"\b(?:sk-[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[abprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35})",
            ),
            rule("session id", r#"(?i)"session"\s*:\s*"[0-9a-f]{8,}""#),
            rule("session id", r#"(?i)mcp-session-id"?\s*[:=]\s*"?[0-9a-f]{16,}"#),
            rule("session id", r"\b[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}\b"),
            rule("IP address", r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b"),
        ];
        let rules = rules.into_iter().collect::<Option<Vec<_>>>()?;
        let local =
            local.iter().map(|n| n.trim().to_lowercase()).filter(|n| n.chars().count() >= 3 && !GENERIC_NAMES.contains(&n.as_str())).collect();
        let allow = allow
            .lines()
            .map(|l| l.split("  #").next().unwrap_or("").trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect();
        Some(Self { rules, local, allow })
    }

    /// What in `text` looks personal.
    pub fn scan(&self, text: &str) -> Vec<Finding> {
        let mut out = Vec::new();
        for (i, line) in text.lines().enumerate() {
            for r in &self.rules {
                // The reported text is the first group when the rule has one (what precedes it only
                // anchors it), else the whole match.
                for c in r.re.captures_iter(line) {
                    let Some(m) = c.get(1).or_else(|| c.get(0)) else { continue };
                    let found = m.as_str().trim_end_matches(|c: char| !c.is_ascii_alphanumeric()).to_string();
                    if !self.allowed(r.what, &found) {
                        out.push(Finding { what: r.what, text: found, line: i + 1 });
                    }
                }
            }
            let lower = line.to_lowercase();
            for n in &self.local {
                if contains_word(&lower, n) {
                    out.push(Finding { what: "this machine's login or host name", text: "<this machine>".into(), line: i + 1 });
                }
            }
        }
        out
    }

    fn allowed(&self, what: &str, found: &str) -> bool {
        if self.allow.iter().any(|a| a == found) {
            return true;
        }
        match what {
            "email address" => found.rsplit('@').next().is_some_and(example_domain),
            "IP address" => found == "127.0.0.1" || found.split('.').any(|o| o.parse::<u16>().map_or(true, |v| v > 255)),
            _ => false,
        }
    }
}

/// The domains reserved for examples (RFC 2606, RFC 6761).
fn example_domain(d: &str) -> bool {
    let d = d.to_lowercase();
    ["example.com", "example.org", "example.net"].iter().any(|e| d == *e || d.ends_with(&format!(".{e}")))
        || [".example", ".test", ".invalid", ".localhost"].iter().any(|t| d.ends_with(t))
}

/// `name` appears in `hay` on its own (not inside a longer word).
fn contains_word(hay: &str, name: &str) -> bool {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    hay.match_indices(name).any(|(i, _)| !word(hay[..i].chars().next_back()) && !word(hay[i + name.len()..].chars().next()))
}

/// This machine's login and host name, when they can be read.
fn local_names() -> Vec<String> {
    let mut names: Vec<String> = ["USER", "LOGNAME", "USERNAME"].iter().filter_map(|v| std::env::var(v).ok()).collect();
    if let Ok(out) = Command::new("hostname").arg("-s").output().or_else(|_| Command::new("hostname").output()) {
        names.push(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    names.retain(|n| !n.is_empty());
    names
}

pub fn check(root: &Path) -> bool {
    let allow = std::fs::read_to_string(root.join(ALLOW)).unwrap_or_default();
    let Some(scanner) = Scanner::new(&allow, &local_names()) else {
        eprintln!("pii-scan: a pattern didn't compile");
        return false;
    };
    let Ok(out) = Command::new("git").args(["ls-files", "-z", "docs", "scripts"]).current_dir(root).output() else {
        eprintln!("pii-scan: `git ls-files` failed");
        return false;
    };
    let (mut bad, mut seen) = (Vec::new(), 0);
    for rel in out.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let rel = String::from_utf8_lossy(rel).into_owned();
        // Text only: images and other binary files don't read as UTF-8.
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else { continue };
        seen += 1;
        for f in scanner.scan(&text) {
            bad.push(format!("{rel}:{}: {} `{}`", f.line, f.what, f.text));
        }
    }
    if !bad.is_empty() {
        eprintln!(
            "pii-scan: personal data in docs/ or scripts/ (use a placeholder such as /Users/<user>, <device> or <token>, or list a deliberate value in {ALLOW} with its reason):\n  {}",
            bad.join("\n  ")
        );
        return false;
    }
    eprintln!("pii-scan: ok ({seen} text files)");
    true
}

#[cfg(test)]
#[path = "pii_scan_tests.rs"]
mod tests;
