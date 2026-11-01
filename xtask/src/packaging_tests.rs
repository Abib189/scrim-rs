//! The packaging files keep what Scrim ships to Scrim's own (Fix AI-4): the Flatpak gets the one
//! permission live sessions need beyond the display and the files people open, and the server
//! installer never fetches anyone else's binaries.

use std::path::Path;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap_or(Path::new("."))
}

/// The `finish-args` of a Flatpak manifest, as written (`--share=network`, …), comments left out.
fn finish_args(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        if line.trim_end() == "finish-args:" {
            inside = true;
            continue;
        }
        if inside {
            let t = line.trim();
            if t.starts_with('#') || t.is_empty() {
                continue;
            }
            // The block ends at the next top-level key.
            if !line.starts_with(' ') {
                break;
            }
            if let Some(arg) = t.strip_prefix("- ") {
                out.push(arg.trim().to_string());
            }
        }
    }
    out
}

#[test]
fn the_flatpak_grants_the_network_live_sessions_need_and_nothing_broader() {
    let want = [
        "--socket=wayland",
        "--socket=fallback-x11",
        "--share=ipc",
        "--device=dri",
        "--filesystem=xdg-pictures",
        "--filesystem=xdg-documents",
        // Live sessions, Share > Online and team libraries reach a server.
        "--share=network",
    ];
    for name in ["design.scrim.app.yml", "design.scrim.app.bundle.yml"] {
        let path = root().join("packaging/linux/flatpak").join(name);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let mut got = finish_args(&text);
        got.sort();
        let mut want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
        want.sort();
        assert_eq!(got, want, "{name}: the Flatpak's permissions");
        for broad in ["--filesystem=home", "--filesystem=host", "--socket=session-bus", "--socket=system-bus", "--device=all", "--share=all"] {
            assert!(!got.iter().any(|a| a == broad), "{name} grants {broad}");
        }
    }
}

#[test]
fn the_server_installer_fetches_only_scrims_own_source() {
    let path = root().join("packaging/server/install.sh");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(text.contains("REPO=\"Abib189/scrim-rs\""), "the installer names Scrim's repository");
    assert!(!text.contains("storytold/figcraft"), "the installer never fetches FigCraft's releases or source");
    // Scrim publishes no server release archives yet: the installer says so instead of downloading.
    assert!(text.contains("publishes no figcraft-server release archives yet"), "the release download refuses with a reason");
    // Every download (a curl command, not the help text it prints) is https only: a redirect to
    // http is refused.
    for line in text.lines().filter(|l| l.contains("curl ") && l.contains(" -f") && l.contains("https://") && !l.trim_start().starts_with('#')) {
        assert!(line.contains("--proto '=https'"), "{}", line.trim());
    }
}
