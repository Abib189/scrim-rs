//! G2's guard: the brand orange can't creep back into interactive code.
//! - **No literal brand orange** (`#ff5a1f` in any spelling) in any file of the code, the assets or
//!   the token sources, outside an explicit allow-list: the token definitions, the app icon and its
//!   generator, text that names the default, and tests that assert the default's value.
//! - **The constants** `theme::ACCENT` / `ACCENT_ON` (the default, not the person's accent) are used
//!   only where tokens are defined. Code that paints reads the live accent (`theme::ink`, `Tokens`)
//!   or, for the brand mark, `theme::BRAND`.

use std::path::{Path, PathBuf};

/// The repository's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every file under `dir` (relative to the root), build output and hidden folders left out.
fn files(dir: &str) -> Vec<PathBuf> {
    fn walk(p: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(p) else { return };
        for e in rd.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root().join(dir), &mut out);
    out
}

fn rel(p: &Path) -> String {
    p.strip_prefix(root()).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// Files that may spell the brand orange out.
const ALLOWED: [&str; 16] = [
    // The token definitions.
    "crates/engine/src/cmd/appearance.rs",
    "crates/ui-egui/src/theme.rs",
    "crates/ui-egui/src/accent.rs",
    // The app icon and its generator.
    "assets/app-icon/",
    "xtask/src/icon.rs",
    // Text that names the default ("Back to Scrim's orange, #ff5a1f").
    "crates/ui-egui/src/appearance_prefs.rs",
    "crates/ui-egui/locales/",
    // Scrim's design-system source (tokens.json and its generators).
    "docs/scrim-transfer/",
    // Tests that assert the default's value (or use it as a document colour).
    "crates/engine/src/cmd/appearance_tests.rs",
    "crates/engine/src/cmd/query/figcraft_tests.rs",
    "crates/mcp/tests/panel.rs",
    "crates/ui-egui/src/accent_tests.rs",
    "crates/ui-egui/src/appearance_prefs_tests.rs",
    "crates/ui-egui/src/accent_ink_tests.rs",
    "crates/ui-egui/src/accent_contrast_tests.rs",
    "crates/ui-egui/src/accent_guard_tests.rs",
];

/// The brand orange's spellings, lower-cased and without spaces (built here so this file doesn't
/// hold them literally).
fn spellings() -> Vec<String> {
    let (r, g, b) = ("ff", "5a", "1f");
    vec![format!("{r}{g}{b}"), format!("0x{r},0x{g},0x{b}"), "255,90,31".into()]
}

#[test]
fn no_brand_orange_literal_outside_the_allow_list() {
    let words = spellings();
    let mut found = Vec::new();
    for dir in ["crates", "apps", "xtask", "assets", "docs/scrim-transfer", "tools", "plugins"] {
        for f in files(dir) {
            let r = rel(&f);
            if ALLOWED.iter().any(|a| r.starts_with(a)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            let squeezed: String = text.to_ascii_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
            if let Some(w) = words.iter().find(|w| squeezed.contains(w.as_str())) {
                found.push(format!("{r}: {w}"));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the brand orange, hard-coded (paint with theme::ink / Tokens, or theme::BRAND for the brand mark):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_default_accent_constants_are_used_only_where_tokens_are_defined() {
    let defined = ["crates/ui-egui/src/theme.rs", "crates/ui-egui/src/accent.rs"];
    let mut found = Vec::new();
    for f in files("crates/ui-egui/src").into_iter().chain(files("apps")) {
        let r = rel(&f);
        if !r.ends_with(".rs") || r.ends_with("_tests.rs") || r.contains("/tests/") || defined.contains(&r.as_str()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        for (i, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            let hit = code.match_indices("ACCENT").any(|(at, _)| {
                let before = code[..at].chars().next_back();
                let after = code[at + "ACCENT".len()..].chars().next();
                let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
                // `ACCENT` or `ACCENT_ON`, not a longer name (`DEFAULT_ACCENT`, `ACCENT_X`).
                !word(before) && (!word(after) || code[at..].starts_with("ACCENT_ON") && !word(code[at + 9..].chars().next()))
            });
            if hit {
                found.push(format!("{r}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "theme::ACCENT / ACCENT_ON is the default, not the person's accent (use theme::ink or Tokens):\n{}", found.join("\n"));
}
