use super::Scanner;

fn scanner(allow: &str, local: &[&str]) -> Scanner {
    let local: Vec<String> = local.iter().map(|s| s.to_string()).collect();
    Scanner::new(allow, &local).expect("the patterns compile")
}

fn kinds(s: &Scanner, text: &str) -> Vec<&'static str> {
    s.scan(text).into_iter().map(|f| f.what).collect()
}

#[test]
fn each_kind_of_personal_data_is_found() {
    let s = scanner("", &[]);
    let cases = [
        ("\"PATH\": \"/Users/jdoe/.nvm/bin\"", "home folder"),
        ("cd /home/jdoe/src", "home folder"),
        (r"C:\Users\jdoe\AppData", "home folder"),
        ("/private/tmp/claude-501/-Users-jdoe-Scrim/scratchpad", "home folder (encoded)"),
        ("ssh build-box.local and copy", "device name"),
        ("mail jdoe@gmail.com", "email address"),
        ("Authorization: Bearer abcdef0123456789xyz", "bearer token"),
        ("key sk-ant-api03-abcdefghijklmnopqrstuv", "API key"),
        ("token ghp_abcdefghijklmnopqrstuvwxyz0123", "API key"),
        (r#"{"session": "8caa29ef", "conn": 1}"#, "session id"),
        ("Mcp-Session-Id: 0123456789abcdef0123456789abcdef", "session id"),
        ("id 3f2504e0-4f89-11d3-9a0c-0305e82c3301", "session id"),
        ("listening on 10.0.0.12:7680", "IP address"),
    ];
    for (text, kind) in cases {
        assert_eq!(kinds(&s, text), vec![kind], "{text}");
    }
}

#[test]
fn placeholders_loopback_and_example_domains_pass() {
    let s = scanner("", &[]);
    for text in [
        "\"PATH\": \"/Users/<user>/.nvm/bin\"",
        "<scratch>/home-f/Documents/Scrim",
        "the machine <device>, the token <token>",
        "Authorization: Bearer <token>",
        "http://127.0.0.1:7680/mcp",
        "root@figcraft.example.com and abuse@example.com",
        "Claude Code's .claude/settings.local.json",
        "~/.local/share/scrim",
        "apps/studio/src/home/useHomeActions.ts",
        "<scratch>/home/Library/Application Support/Claude",
        "version 1.2.3, port 7680",
        "\"session\": \"<session-a>\"",
    ] {
        assert!(s.scan(text).is_empty(), "{text}: {:?}", s.scan(text));
    }
}

#[test]
fn the_allow_list_lets_deliberate_values_through_and_nothing_else() {
    let allow = "# deliberate values\n192.168.1.20  # an example LAN address in the protocol doc\nFA07D3AB-4037-4826-BA91-B6E37FFAFC4E  # the WiX UpgradeCode\n";
    let s = scanner(allow, &[]);
    assert!(s.scan("figcraft://join?host=192.168.1.20:47900").is_empty());
    assert!(s.scan("UpgradeCode FA07D3AB-4037-4826-BA91-B6E37FFAFC4E").is_empty());
    assert_eq!(kinds(&s, "host=192.168.1.21"), vec!["IP address"], "only the listed value");
}

#[test]
fn this_machines_own_names_are_found_without_being_written_down() {
    let s = scanner("", &["jdoe", "jdoe-mbp", "runner", "ci"]);
    assert_eq!(kinds(&s, "built on JDOE-MBP by jdoe"), vec!["this machine's login or host name", "this machine's login or host name"]);
    assert!(s.scan("the test runner, in ci, by jdoes").is_empty(), "generic names and longer words don't count");
    assert!(s.scan("built on jdoe-mbp").iter().all(|f| f.text == "<this machine>"), "the name isn't repeated in the report");
}

#[test]
fn findings_carry_their_line() {
    let s = scanner("", &[]);
    let f = s.scan("fine\nstill fine\nsee /Users/jdoe/x");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].line, 3);
    assert_eq!(f[0].text, "/Users/jdoe");
}
