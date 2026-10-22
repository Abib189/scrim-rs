//! Live scripts never touch the person's own settings (Fix Q3, AGENTS.md › Scrim's rules 9).
//! Every script under `scripts/` and `docs/` (`*.py`, `*.sh`) that changes a setting must name
//! `SCRIM_SETTINGS_DIR`: it starts the app with it (a scratch settings folder), or checks it
//! (`app.settings-dir` answers `override: true`) before it changes anything.
//! - **Changes a setting:** sends a `pref.*` command, or writes `state.json` itself.

use std::path::Path;
use std::process::Command;

/// The override every such script must name.
const OVERRIDE: &str = "SCRIM_SETTINGS_DIR";

pub fn check(root: &Path) -> bool {
    let Ok(out) = Command::new("git").args(["ls-files", "-z", "scripts", "docs"]).current_dir(root).output() else {
        eprintln!("live-scripts: `git ls-files` failed");
        return false;
    };
    let mut bad = Vec::new();
    let mut seen = 0;
    for rel in out.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let rel = String::from_utf8_lossy(rel).into_owned();
        if !(rel.ends_with(".py") || rel.ends_with(".sh")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else { continue };
        seen += 1;
        if let Some(why) = problem(&text) {
            bad.push(format!("{rel}: {why}"));
        }
    }
    if !bad.is_empty() {
        eprintln!(
            "live-scripts: scripts that change settings must run against a scratch settings folder ({OVERRIDE}; AGENTS.md › Scrim's rules 9):\n  {}",
            bad.join("\n  ")
        );
        return false;
    }
    eprintln!("live-scripts: ok ({seen} scripts)");
    true
}

/// Why `text` (a script) breaks the rule, if it does.
pub fn problem(text: &str) -> Option<&'static str> {
    let sets_pref = ["'pref.", "\"pref."].iter().any(|q| text.contains(q));
    let writes_state = text.contains("state.json");
    if !(sets_pref || writes_state) || text.contains(OVERRIDE) {
        return None;
    }
    Some(if sets_pref { "sends pref.* commands without SCRIM_SETTINGS_DIR" } else { "writes state.json without SCRIM_SETTINGS_DIR" })
}

#[cfg(test)]
mod tests {
    use super::problem;

    #[test]
    fn a_script_that_changes_settings_must_name_the_override() {
        assert!(problem("a.run('pref.agentGrace', {'seconds': 15})").is_some());
        assert!(problem(r#"a.run("pref.revealSpeed", {})"#).is_some());
        assert!(problem("json.dump(prefs, open(home + '/state.json', 'w'))").is_some());
        assert!(problem("env['SCRIM_SETTINGS_DIR'] = d\na.run('pref.agentGrace', {'seconds': 15})").is_none());
        assert!(problem("a.run('file.new')").is_none(), "no setting changed");
    }
}
