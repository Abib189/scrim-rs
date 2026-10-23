//! `cargo xtask bundle-macos` (R4, `docs/design/macos-bundle.md`): a double-clickable Scrim.app for
//! this Mac, at `<target>/bundle/Scrim.app`.
//! - **Contents/MacOS/figcraft:** the windowed app, a release build for this Mac. Connect Claude
//!   Desktop writes this path for the bridge (`figcraft mcp-bridge`). Built with
//!   `FIGCRAFT_BUILD_SHA` set to the commit (Fix Y3), as `packaging/env.sh` does for the release:
//!   its `--version` says "Scrim <version> (<sha>)", and the bundle isn't made if it doesn't.
//! - **Contents/Info.plist:** the release template (`packaging/macos/Info.plist.in`): Scrim,
//!   `design.scrim.app`, NSHighResolutionCapable, with the version from Cargo, the minimum macOS
//!   the build targets, and this executable's name.
//! - **Contents/Resources/Scrim.icns:** made from the placeholder icon's PNGs. The one swap point
//!   for the real mark is `assets/app-icon/scrim/` (`cargo xtask icon` writes it).
//! - **Signed ad hoc** (`codesign -s -`): it runs on this Mac. It isn't notarised; another Mac's
//!   Gatekeeper refuses it until opened from the context menu. Nothing here changes Gatekeeper
//!   or a file's quarantine.
//! - The signed, notarised release is `packaging/macos/package.sh`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The icon sizes an `.iconset` holds (each also at 2×).
const ICON_SIZES: [u32; 5] = [16, 32, 128, 256, 512];

fn run(cmd: &mut Command) -> bool {
    eprintln!("$ {cmd:?}");
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// The macOS version the build targets: `MACOSX_DEPLOYMENT_TARGET`, else the compiler's default
/// for this Mac (`rustc --print deployment-target`).
pub fn deployment_target() -> Option<String> {
    if let Ok(v) = std::env::var("MACOSX_DEPLOYMENT_TARGET")
        && !v.trim().is_empty()
    {
        return Some(v.trim().to_string());
    }
    let out = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into())).args(["--print", "deployment-target"]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.trim().rsplit('=').next().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string)
}

/// The Info.plist from the release template: `version` (full), the executable `exe`, and the
/// minimum macOS `min`. `None` when the template no longer has what is replaced.
pub fn info_plist(template: &str, version: &str, sha: &str, exe: &str, min: &str) -> Option<String> {
    let short = version.split(['-', '+']).next().unwrap_or(version);
    let exe_from = "<key>CFBundleExecutable</key>\n  <string>Scrim</string>";
    let min_from = "<key>LSMinimumSystemVersion</key>\n  <string>11.0</string>";
    if !template.contains(exe_from) || !template.contains(min_from) || !template.contains("@SHORT_VERSION@") {
        return None;
    }
    Some(
        template
            .replace(exe_from, &format!("<key>CFBundleExecutable</key>\n  <string>{exe}</string>"))
            .replace(min_from, &format!("<key>LSMinimumSystemVersion</key>\n  <string>{min}</string>"))
            .replace("@VERSION@", version)
            .replace("@SHORT_VERSION@", short)
            .replace("@BUILD_SHA@", sha),
    )
}

fn target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"))
}

/// `Contents/Resources/Scrim.icns` from the placeholder icon's PNGs.
fn icon(root: &Path, work: &Path, resources: &Path) -> bool {
    let icons = root.join("assets/app-icon/scrim");
    let set = work.join("Scrim.iconset");
    let _ = std::fs::remove_dir_all(&set);
    if std::fs::create_dir_all(&set).is_err() {
        return false;
    }
    for s in ICON_SIZES {
        let copied = std::fs::copy(icons.join(format!("scrim-{s}.png")), set.join(format!("icon_{s}x{s}.png")))
            .and_then(|_| std::fs::copy(icons.join(format!("scrim-{}.png", s * 2)), set.join(format!("icon_{s}x{s}@2x.png"))));
        if let Err(e) = copied {
            eprintln!("bundle-macos: the icon's {s} px PNGs ({}): {e}", icons.display());
            return false;
        }
    }
    run(Command::new("iconutil").arg("-c").arg("icns").arg("-o").arg(resources.join("Scrim.icns")).arg(&set))
}

/// The commit the bundle is built from: `FIGCRAFT_BUILD_SHA` when it's set (as `packaging/env.sh`
/// leaves it), else `git rev-parse HEAD`; `None` when neither says.
pub fn build_sha(from_env: Option<String>, from_git: impl FnOnce() -> Option<String>) -> Option<String> {
    from_env.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).or_else(|| from_git().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
}

/// The binary's `--version` names the commit: "Scrim <version> (<sha>)".
pub fn version_has_commit(output: &str, sha: &str) -> bool {
    !sha.is_empty() && output.trim().starts_with("Scrim ") && output.contains(&format!("({sha})"))
}

pub fn bundle(root: &Path, cargo: &mut Command) -> bool {
    if !cfg!(target_os = "macos") {
        eprintln!("bundle-macos: builds a macOS app, on macOS only");
        return false;
    }
    let Some(min) = deployment_target() else {
        eprintln!("bundle-macos: couldn't tell the minimum macOS (set MACOSX_DEPLOYMENT_TARGET)");
        return false;
    };
    // The commit, in the binary and the Info.plist alike (Fix Y3).
    let git =
        || Command::new("git").args(["rev-parse", "HEAD"]).current_dir(root).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string());
    let Some(sha) = build_sha(std::env::var("FIGCRAFT_BUILD_SHA").ok(), git) else {
        eprintln!("bundle-macos: couldn't tell the commit (set FIGCRAFT_BUILD_SHA, or build in the git checkout)");
        return false;
    };
    // The binary and the Info.plist name the same minimum.
    if !run(cargo.args(["build", "--release", "--locked", "-p", "figcraft"]).env("MACOSX_DEPLOYMENT_TARGET", &min).env("FIGCRAFT_BUILD_SHA", &sha)) {
        return false;
    }
    let target = target_dir(root);
    let work = target.join("bundle");
    let app = work.join("Scrim.app");
    let (macos, resources) = (app.join("Contents/MacOS"), app.join("Contents/Resources"));
    let _ = std::fs::remove_dir_all(&app);
    if std::fs::create_dir_all(&macos).and_then(|()| std::fs::create_dir_all(&resources)).is_err() {
        eprintln!("bundle-macos: couldn't create {}", app.display());
        return false;
    }
    let exe = macos.join("figcraft");
    if let Err(e) = std::fs::copy(target.join("release/figcraft"), &exe) {
        eprintln!("bundle-macos: copying the app: {e}");
        return false;
    }
    let Ok(template) = std::fs::read_to_string(root.join("packaging/macos/Info.plist.in")) else {
        eprintln!("bundle-macos: packaging/macos/Info.plist.in is missing");
        return false;
    };
    let version = crate::workspace_version();
    let Some(plist) = info_plist(&template, &version, &sha, "figcraft", &min) else {
        eprintln!("bundle-macos: Info.plist.in no longer has the executable, minimum macOS or version this replaces");
        return false;
    };
    let info = app.join("Contents/Info.plist");
    if std::fs::write(&info, plist).and_then(|()| std::fs::write(app.join("Contents/PkgInfo"), "APPL????")).is_err() {
        return false;
    }
    if !run(Command::new("plutil").arg("-lint").arg(&info)) || !icon(root, &work, &resources) {
        return false;
    }
    // Ad hoc, inside out: the executable, then the bundle; then check the seal.
    let sign = |path: &Path| run(Command::new("codesign").args(["--force", "--sign", "-", "--timestamp=none"]).arg(path));
    if !sign(&exe) || !sign(&app) || !run(Command::new("codesign").args(["--verify", "--strict", "--verbose=2"]).arg(&app)) {
        return false;
    }
    // The binary says which commit it is, or this isn't the bundle it should be.
    let said = Command::new(&exe).arg("--version").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();
    if !version_has_commit(&said, &sha) {
        eprintln!("bundle-macos: the bundle's figcraft --version doesn't name the commit {sha}: {:?}", said.trim());
        return false;
    }
    eprintln!("bundle-macos: {} ({}, macOS {min} or later, signed ad hoc)", app.display(), said.trim());
    true
}

#[cfg(test)]
#[path = "bundle_macos_tests.rs"]
mod tests;
