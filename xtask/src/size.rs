//! No source file over `LIMIT` lines (AGENTS.md › Scrim's rules 3). Tests (`*_tests.rs`, `tests/`),
//! generated and data files are exempt. Files that were already longer when the fork was made are
//! frozen in `xtask/size-allowlist.txt` (`<lines> <path>`): they may shrink, never grow, and an
//! entry whose file is back under the limit must be removed.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub const LIMIT: usize = 1000;
const ALLOWLIST: &str = "xtask/size-allowlist.txt";

pub fn check(root: &Path) -> bool {
    let Ok(out) = Command::new("git").args(["ls-files", "-z", "*.rs"]).current_dir(root).output() else {
        eprintln!("size: `git ls-files` failed");
        return false;
    };
    let allowed = parse_allowlist(&std::fs::read_to_string(root.join(ALLOWLIST)).unwrap_or_default());
    let mut lengths = BTreeMap::new();
    for rel in out.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let rel = String::from_utf8_lossy(rel).replace('\\', "/");
        if exempt(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else { continue };
        lengths.insert(rel, text.lines().count());
    }
    let problems = problems(&lengths, &allowed);
    if !problems.is_empty() {
        eprintln!("size: source files stay under {LIMIT} lines (AGENTS.md › Scrim's rules 3):\n  {}", problems.join("\n  "));
        return false;
    }
    eprintln!("size: ok ({} files, {} frozen)", lengths.len(), allowed.len());
    true
}

fn exempt(rel: &str) -> bool {
    rel.ends_with("_tests.rs") || rel.ends_with("_test.rs") || rel.contains("/tests/") || rel.contains("/generated/")
}

fn parse_allowlist(text: &str) -> BTreeMap<String, usize> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (n, path) = l.split_once(' ')?;
            Some((path.trim().to_string(), n.parse().ok()?))
        })
        .collect()
}

fn problems(lengths: &BTreeMap<String, usize>, allowed: &BTreeMap<String, usize>) -> Vec<String> {
    let mut out = Vec::new();
    for (path, &n) in lengths {
        match allowed.get(path) {
            Some(&cap) if n > cap => out.push(format!("{path}: {n} lines, frozen at {cap}: it may shrink, never grow")),
            Some(_) if n <= LIMIT => out.push(format!("{path}: {n} lines is under {LIMIT}: remove it from {ALLOWLIST}")),
            Some(_) => {}
            None if n > LIMIT => out.push(format!("{path}: {n} lines (limit {LIMIT}): split it by concern")),
            None => {}
        }
    }
    for path in allowed.keys().filter(|p| !lengths.contains_key(*p)) {
        out.push(format!("{path}: listed in {ALLOWLIST} but not a source file any more: remove the entry"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(v: &[(&str, usize)]) -> BTreeMap<String, usize> {
        v.iter().map(|(p, n)| (p.to_string(), *n)).collect()
    }

    #[test]
    fn new_files_over_the_limit_and_frozen_files_that_grow_are_refused() {
        let allowed = map(&[("big.rs", 1500), ("shrunk.rs", 1200), ("gone.rs", 2000)]);
        let lengths = map(&[("ok.rs", 999), ("new.rs", 1001), ("big.rs", 1501), ("shrunk.rs", 900)]);
        let p = problems(&lengths, &allowed);
        assert_eq!(p.len(), 4, "{p:?}");
        assert!(p.iter().any(|s| s.starts_with("new.rs: 1001 lines")));
        assert!(p.iter().any(|s| s.starts_with("big.rs: 1501 lines, frozen at 1500")));
        assert!(p.iter().any(|s| s.starts_with("shrunk.rs: 900 lines is under")));
        assert!(p.iter().any(|s| s.starts_with("gone.rs: listed")));
        assert!(problems(&map(&[("big.rs", 1400)]), &map(&[("big.rs", 1500)])).is_empty());
    }

    #[test]
    fn the_allowlist_reads_counts_and_paths_and_skips_comments() {
        let a = parse_allowlist("# comment\n\n1500 crates/a.rs\n  2000 crates/b c.rs  \nnot-a-number x.rs\n");
        assert_eq!(a, map(&[("crates/a.rs", 1500), ("crates/b c.rs", 2000)]));
    }

    #[test]
    fn tests_and_generated_files_are_exempt() {
        assert!(exempt("crates/engine/src/file_access_tests.rs"));
        assert!(exempt("crates/doc/tests/round_trip.rs"));
        assert!(!exempt("crates/engine/src/lib.rs"));
    }
}
