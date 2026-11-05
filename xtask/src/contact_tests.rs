//! Fix R-6: the maintainers' address has one place, `figcraft_engine::project::MAINTAINER_CONTACT`.
//! The About dialog and the Info.plist take it from there, `package.sh` reads it from there, and
//! NOTICE and README carry it as it is; no other file in the repository spells it out.

use figcraft_engine::project::MAINTAINER_CONTACT;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the workspace").to_path_buf()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// Where the address may be written out: its definition, and the two texts that can't take a
/// constant.
const SPELLED_OUT: &[&str] = &["crates/engine/src/project.rs", "README.md", "NOTICE"];

#[test]
fn notice_readme_about_and_the_plist_show_the_one_address() {
    assert!(MAINTAINER_CONTACT.contains('@'));
    for doc in ["README.md", "NOTICE"] {
        assert!(read(doc).contains(&format!("Contact the maintainers: {MAINTAINER_CONTACT}")), "{doc} names the maintainers' address");
    }
    assert!(read("crates/ui-egui/src/dialogs.rs").contains("figcraft_engine::project::MAINTAINER_CONTACT"), "the About dialog shows it");
    let template = read("packaging/macos/Info.plist.in");
    let plist = crate::bundle_macos::info_plist(&template, "0.7.0", "abc", "figcraft", "11.0").expect("the template");
    assert!(plist.contains(&format!("MIT OR Apache-2.0. Contact: {MAINTAINER_CONTACT}</string>")), "the copyright line");
    assert!(read("packaging/macos/package.sh").contains("MAINTAINER_CONTACT"), "package.sh reads it from the constant");
}

#[test]
fn no_other_file_spells_the_address_out() {
    let files = std::process::Command::new("git").args(["ls-files", "-z"]).current_dir(root()).output().expect("git ls-files");
    let mut found = Vec::new();
    for f in String::from_utf8_lossy(&files.stdout).split('\0').filter(|f| !f.is_empty()) {
        let Ok(text) = std::fs::read_to_string(root().join(f)) else { continue };
        if text.contains(MAINTAINER_CONTACT) && !SPELLED_OUT.contains(&f) {
            found.push(f.to_string());
        }
    }
    assert!(found.is_empty(), "spelled out outside its one place (use figcraft_engine::project::MAINTAINER_CONTACT): {found:?}");
}
