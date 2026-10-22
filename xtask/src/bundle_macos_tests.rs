use super::info_plist;

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
