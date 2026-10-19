//! The ArtCraft marks stay out of this fork (docs/FORK.md › Licences). FigCraft's brand terms let a
//! fork say, in plain text, that it is "based on FigCraft by the ArtCraft team" and nothing more:
//! - no brand files (the old `docs/brand/`, any `artcraft-*` file);
//! - no ArtCraft links (their site, their Discord, their hosted logos), anywhere;
//! - in what ships (`crates/`, `apps/`, `packaging/`), the name only inside that phrase.
//!
//! Docs and the licence files are left to review: they may name ArtCraft to explain the trademark.

use std::path::Path;
use std::process::Command;

/// The phrase the brand terms allow in shipped text.
const ALLOWED: &str = "FigCraft by the ArtCraft team";

pub fn check(root: &Path) -> bool {
    let Ok(out) = Command::new("git").args(["ls-files", "-z"]).current_dir(root).output() else {
        eprintln!("brand: `git ls-files` failed");
        return false;
    };
    let mut bad = Vec::new();
    for rel in out.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let rel = String::from_utf8_lossy(rel).replace('\\', "/");
        if is_brand_file(&rel) {
            bad.push(format!("{rel}: an ArtCraft brand file"));
            continue;
        }
        // This check and the fork's own notes quote what they forbid.
        if rel == "xtask/src/brand.rs" || rel == "docs/FORK.md" || rel.starts_with("docs/scrim-transfer/") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else { continue };
        let shipped = ["crates/", "apps/", "packaging/"].iter().any(|p| rel.starts_with(p));
        let catalog = rel.contains("/locales/") && rel.ends_with(".toml");
        for (n, line) in text.lines().enumerate() {
            // A catalog line is `"English key" = "translation"`: the key decides.
            let line = if catalog { line.split(" = ").next().unwrap_or(line) } else { line };
            if let Some(why) = refusal(line, shipped) {
                bad.push(format!("{rel}:{}: {why}: {}", n + 1, line.trim()));
            }
        }
    }
    if !bad.is_empty() {
        eprintln!("brand: the ArtCraft marks must stay out of this fork (docs/FORK.md):\n  {}", bad.join("\n  "));
        return false;
    }
    eprintln!("brand: ok");
    true
}

fn is_brand_file(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel).to_ascii_lowercase();
    rel.starts_with("docs/brand/") || name.starts_with("artcraft-")
}

/// Why `line` may not be in the fork, if it may not.
fn refusal(line: &str, shipped: bool) -> Option<&'static str> {
    let l = line.to_ascii_lowercase();
    if l.contains("getartcraft") || l.contains("discord.gg/artcraft") || l.contains("/docs/brand/artcraft") {
        return Some("an ArtCraft link");
    }
    if shipped && line.matches("ArtCraft").count() > line.matches(ALLOWED).count() + line.matches("ArtCraft product").count() {
        return Some("the ArtCraft name outside \"based on FigCraft by the ArtCraft team\"");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_refused_everywhere_and_the_name_only_in_shipped_text() {
        assert!(refusal("see https://getartcraft.com/apps", false).is_some());
        assert!(refusal("[Discord](https://discord.gg/artcraft)", false).is_some());
        assert!(refusal("Made by the ArtCraft team and community.", true).is_some());
        assert!(refusal("Made by the ArtCraft team and community.", false).is_none());
        assert!(refusal("Based on FigCraft by the ArtCraft team; not an ArtCraft product.", true).is_none());
        assert!(refusal("Copyright (c) 2026 ArtCraft Team and the FigCraft contributors", false).is_none());
        assert!(refusal("plain text", true).is_none());
    }

    #[test]
    fn brand_files_are_found_by_folder_or_name() {
        assert!(is_brand_file("docs/brand/LICENSE-brand.txt"));
        assert!(is_brand_file("assets/artcraft-mark.svg"));
        assert!(!is_brand_file("assets/app-icon/figcraft.svg"));
    }
}
