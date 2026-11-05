//! Fix R-1: the app builds on every platform it ships to. A module that uses a macOS-only crate is
//! declared under `#[cfg(target_os = "macos")]` (or has its own path for other platforms), and code
//! on every platform doesn't reach into a
//! module that only macOS has. (A module once moved the gate off `apple_events` onto the module
//! above it, so a Linux build failed twice: `apple_events` named a macOS-only crate, and
//! `agent_runs`, used everywhere, wasn't built.) Checked by reading the sources, so it runs on the
//! macOS CI that builds the app; the Linux target needn't be installed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const GATE: &str = "#[cfg(target_os = \"macos\")]";

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The crates `Cargo.toml` lists for macOS only (not in any other dependency table), as Rust
/// names them.
fn macos_only_crates() -> BTreeSet<String> {
    let toml = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"));
    let mut table = String::new();
    let (mut mac, mut other) = (BTreeSet::new(), BTreeSet::new());
    for line in toml.lines().map(str::trim) {
        if line.starts_with('[') {
            table = line.to_string();
            continue;
        }
        let Some((name, _)) = line.split_once('=') else { continue };
        let name = name.trim().replace('-', "_");
        if name.is_empty() || name.starts_with('#') || !table.contains("dependencies") || table.contains("build-dependencies") {
            continue;
        }
        if table == "[target.'cfg(target_os = \"macos\")'.dependencies]" {
            mac.insert(name)
        } else {
            other.insert(name)
        };
    }
    mac.difference(&other).cloned().collect()
}

/// The modules `main.rs` declares (tests aside), and whether each is under the macOS gate.
fn modules(main: &str) -> Vec<(String, bool)> {
    let lines: Vec<&str> = main.lines().map(str::trim).collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(name) = line.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) else { continue };
        let attr = i.checked_sub(1).and_then(|j| lines.get(j)).copied().unwrap_or("");
        if attr.contains("test") {
            continue;
        }
        out.push((name.to_string(), attr == GATE));
    }
    out
}

/// Lines of `text` naming `pat` with no macOS gate on them or the two lines above.
fn ungated_uses(text: &str, pat: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(pat) && !l.trim_start().starts_with("//"))
        .filter(|(i, _)| !lines[i.saturating_sub(2)..=*i].iter().any(|l| l.contains(GATE)))
        .map(|(i, l)| format!("{}: {}", i + 1, l.trim()))
        .collect()
}

#[test]
fn a_module_using_a_macos_only_crate_is_declared_for_macos_only() {
    let crates = macos_only_crates();
    assert!(crates.contains("fmv_macos_events"), "the macOS-only crates are read: {crates:?}");
    let mut wrong = Vec::new();
    for (name, gated) in modules(&read(&src().join("main.rs"))) {
        let text = read(&src().join(format!("{name}.rs")));
        // A module may instead gate its macOS code itself, with a path for other platforms
        // (`app_nap`: App Nap is macOS's; elsewhere it does nothing).
        let own_fallback = text.contains("#[cfg(not(target_os = \"macos\"))]");
        if let Some(c) = crates.iter().find(|c| text.contains(&format!("{c}::")))
            && !gated
            && !own_fallback
        {
            wrong.push(format!("`mod {name};` uses {c} but isn't under {GATE}, and has no path for other platforms"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn code_on_every_platform_reaches_macos_only_modules_only_under_the_gate() {
    let main = read(&src().join("main.rs"));
    let mods = modules(&main);
    let gated: Vec<&str> = mods.iter().filter(|(_, g)| *g).map(|(n, _)| n.as_str()).collect();
    let mut wrong = Vec::new();
    for g in &gated {
        for l in ungated_uses(&main, &format!("{g}::")) {
            wrong.push(format!("main.rs {l}"));
        }
        for (name, _) in mods.iter().filter(|(_, is)| !is) {
            for l in ungated_uses(&read(&src().join(format!("{name}.rs"))), &format!("crate::{g}::")) {
                wrong.push(format!("{name}.rs {l}"));
            }
        }
    }
    assert!(wrong.is_empty(), "macOS-only modules {gated:?} reached from code built everywhere:\n{wrong:#?}");
}
