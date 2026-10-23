use super::{build_sha, info_plist, version_has_commit};

fn template() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the workspace");
    std::fs::read_to_string(root.join("packaging/macos/Info.plist.in")).expect("the release template")
}

#[test]
fn the_plist_names_scrim_its_executable_version_and_minimum_macos() {
    let p = info_plist(&template(), "0.7.0-rc.1", "abc1234", "figcraft", "11.0").expect("the template has what is replaced");
    for want in [
        "<key>CFBundleName</key>\n  <string>Scrim</string>",
        "<key>CFBundleIdentifier</key>\n  <string>design.scrim.app</string>",
        "<key>CFBundleExecutable</key>\n  <string>figcraft</string>",
        "<key>CFBundleShortVersionString</key>\n  <string>0.7.0</string>",
        "<key>ScrimVersion</key>\n  <string>0.7.0-rc.1</string>",
        "<key>ScrimBuildCommit</key>\n  <string>abc1234</string>",
        "<key>LSMinimumSystemVersion</key>\n  <string>11.0</string>",
        "<key>NSHighResolutionCapable</key>\n  <true/>",
        "<key>CFBundleIconFile</key>\n  <string>Scrim</string>",
    ] {
        assert!(p.contains(want), "missing {want:?}");
    }
    assert!(!p.contains('@'), "every placeholder replaced");
}

#[test]
fn another_minimum_macos_is_written_as_given() {
    let p = info_plist(&template(), "0.7.0", "x", "figcraft", "12.3").expect("replaced");
    assert!(p.contains("<key>LSMinimumSystemVersion</key>\n  <string>12.3</string>"));
}

#[test]
fn a_template_without_what_is_replaced_is_refused() {
    assert!(info_plist("<plist><dict></dict></plist>", "0.7.0", "x", "figcraft", "11.0").is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn the_written_plist_passes_plutil() {
    let p = info_plist(&template(), "0.7.0", "abc1234", "figcraft", "11.0").expect("replaced");
    let path = std::env::temp_dir().join(format!("scrim-bundle-test-{}.plist", std::process::id()));
    std::fs::write(&path, p).expect("written");
    let ok = std::process::Command::new("plutil").arg("-lint").arg(&path).status().expect("plutil runs").success();
    let _ = std::fs::remove_file(&path);
    assert!(ok);
}

#[cfg(target_os = "macos")]
#[test]
fn the_deployment_target_is_read_from_the_compiler() {
    let min = super::deployment_target().expect("rustc names one");
    assert!(min.split('.').all(|p| p.parse::<u32>().is_ok()), "{min}");
}

/// Fix Y3: the commit comes from `FIGCRAFT_BUILD_SHA` when set, else git; the bundle is refused
/// when the binary's `--version` doesn't name it.
#[test]
fn the_commit_comes_from_the_environment_or_git_and_the_version_must_name_it() {
    let sha = "d60f317aca72cdcc8e73f5738b42b3a4467bc884";
    assert_eq!(build_sha(Some(format!("{sha}\n")), || None).as_deref(), Some(sha), "the release scripts' variable first");
    assert_eq!(build_sha(Some("  ".into()), || Some(format!("{sha}\n"))).as_deref(), Some(sha), "empty: git");
    assert_eq!(build_sha(None, || Some("".into())), None, "neither: no bundle");
    assert!(version_has_commit(&format!("Scrim 0.7.0 ({sha})\n"), sha));
    assert!(!version_has_commit("Scrim 0.7.0\n", sha), "no commit: refused");
    assert!(!version_has_commit("Scrim 0.7.0 (0000000)\n", sha), "another commit: refused");
    assert!(!version_has_commit(&format!("Scrim 0.7.0 ({sha})"), ""), "an empty commit is no commit");
}
