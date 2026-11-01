//! A library link is stored in the document, so it is followed only on this computer
//! ([`crate::library_server`]): a path on another server would make opening someone else's file
//! contact that server, and on Windows send the credentials of whoever opened it.

use crate::Session;

#[test]
fn a_library_is_opened_only_from_this_computer() {
    let mut s = Session::new();
    for path in [
        r"\\attacker.example\share\lib.scrim",
        "//attacker.example/share/lib.scrim",
        "file://attacker.example/share/lib.scrim",
        r"\\?\UNC\attacker.example\share\lib.scrim",
        r"\\.\pipe\lib",
        "https://attacker.example/lib.scrim",
    ] {
        let e = s.fetch_library(path, false).expect_err(&format!("{path} must be refused"));
        assert!(e.to_string().contains("only from this computer"), "{path}: {e}");
    }
}

#[test]
fn a_local_library_path_is_still_followed() {
    let mut s = Session::new();
    // Refused for being absent, not for where it is: the gate let it through.
    let missing = std::env::temp_dir().join("figcraft-no-such-library.scrim");
    let e = s.fetch_library(&missing.to_string_lossy(), false).expect_err("the file does not exist");
    assert!(!e.to_string().contains("only from this computer"), "a local path must not be refused as remote: {e}");
}

/// Paths a file names that are off this computer, on every platform: network shares in every
/// spelling, device paths and reserved device names (upstream FigCraft b3e65c89, audit #1056).
const OFF_THIS_COMPUTER: &[&str] = &[
    r"\\attacker.example\share\lib.scrim",
    "//attacker.example/share/lib.scrim",
    r"\/attacker.example/share/lib.scrim",
    "file://attacker.example/share/lib.scrim",
    "file:////attacker.example/share/lib.scrim",
    r"\\?\UNC\attacker.example\share\lib.scrim",
    r"\\.\pipe\lib",
    r"\\?\GLOBALROOT\Device\Mup\attacker.example\share\lib.scrim",
    r"\\.\C:\lib.scrim",
    "CON",
    "NUL.scrim",
    r"C:\Users\me\COM1.scrim",
];

#[test]
fn the_library_cache_refuses_a_path_off_this_computer_before_touching_it() {
    // The Assets panel reads the libraries a file links through the cache, without going through
    // fetch_library: the cache must refuse such a path before its stat.
    let mut cache = crate::library::Cache::default();
    for path in OFF_THIS_COMPUTER {
        let e = cache.load(path).expect_err(&format!("{path} must be refused"));
        assert!(e.to_string().contains("only from this computer"), "{path}: {e}");
        let e = cache.load_limited(path, 1024).expect_err(&format!("{path} must be refused"));
        assert!(e.to_string().contains("only from this computer"), "{path}: {e}");
    }
}

#[test]
fn the_path_check_reads_windows_and_unix_paths_alike_on_every_platform() {
    for path in OFF_THIS_COMPUTER {
        assert!(!crate::library::on_this_computer(path), "{path} is off this computer");
    }
    for path in [
        r"C:\Users\me\lib.scrim",
        "C:/Users/me/lib.scrim",
        "/home/me/lib.scrim",
        "/Users/me/Design System.scrim",
        "lib.scrim",
        "file:///home/me/lib.scrim",
    ] {
        assert!(crate::library::on_this_computer(path), "{path} is on this computer");
    }
}
