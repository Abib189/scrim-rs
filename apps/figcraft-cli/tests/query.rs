//! `figcraft-cli query` and `find` (Port 5) end to end on a transfer fixture: the output, the
//! JSON, and exit codes 1 (invalid query), 2 (usage) and 3 (the file). The argument rules are
//! tested against Scrim's CLI cases in the engine (`cmd/query/scrim_cases_tests.rs`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;

fn cli(args: &[&str]) -> (i32, String, String) {
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/fixtures/fig/stage-b.fig");
    let args: Vec<&str> = args.iter().map(|a| if *a == "<file>" { fixture } else { a }).collect();
    let out = Command::new(env!("CARGO_BIN_EXE_figcraft-cli")).arg("--no-system-fonts").args(&args).output().unwrap();
    (out.status.code().unwrap(), String::from_utf8(out.stdout).unwrap(), String::from_utf8(out.stderr).unwrap())
}

#[test]
fn query_and_find_print_the_matches() {
    let (code, out, err) = cli(&["query", "<file>", "//TEXT"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(out.starts_with("2 matches\n  TEXT      Label  ("), "{out}");
    let (code, out, _) = cli(&["find", "<file>", "--type", "frame", "--name", "Row"]);
    assert_eq!((code, out.lines().next()), (0, Some("1 match")));
    let (code, out, _) = cli(&["query", "<file>", "//*", "--limit=1", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!((code, v["returned"].as_u64(), v["truncated"].as_bool()), (0, Some(1), Some(true)));
    // The tool's key order.
    assert!(out.starts_with("{\n  \"query\": \"//*\",\n  \"total\": "), "{out}");
}

#[test]
fn errors_exit_by_kind() {
    let (code, _, err) = cli(&["query", "<file>", "//TEXT/.."]);
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "figcraft-cli: invalid query: '..' (parent) is not supported: queries walk down from the document with / and // (at character 8)\n"
    );
    let (code, _, err) = cli(&["find", "<file>", "--type", "BOX"]);
    assert_eq!(code, 2);
    assert!(err.starts_with("figcraft-cli: --type is one of FRAME, GROUP, SECTION"), "{err}");
    let (code, out, _) = cli(&["query", "no-such.fig", "//*", "--json"]);
    assert_eq!(code, 3);
    assert!(out.contains("\"code\": \"not_found\""), "{out}");
}
