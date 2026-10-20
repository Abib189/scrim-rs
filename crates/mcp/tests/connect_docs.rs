//! The connect commands the app offers are the ones `docs/mcp.md` and the README show, and the
//! server announces itself as `scrim` (A3, C1).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use figcraft_engine::agents::{claude_add_command, connect_command};

const MCP_DOC: &str = include_str!("../../../docs/mcp.md");
const README: &str = include_str!("../../../README.md");

#[test]
fn the_apps_connect_commands_are_the_documented_ones() {
    for cmd in [connect_command(None), connect_command(Some(7979))] {
        assert!(MCP_DOC.contains(&cmd), "docs/mcp.md doesn't show `{cmd}`");
        assert!(cmd.contains("scrim mcp") && !cmd.contains("figcraft"), "{cmd}");
    }
    assert!(!MCP_DOC.contains("claude mcp add figcraft"), "docs/mcp.md still shows the old command");
}

#[test]
fn the_http_connect_command_is_the_documented_one() {
    let plain = claude_add_command(7600, None);
    let with_token = claude_add_command(7600, Some("<token>"));
    for (doc, name) in [(MCP_DOC, "docs/mcp.md"), (README, "README.md")] {
        assert!(doc.contains(&plain), "{name} doesn't show `{plain}`");
    }
    assert!(MCP_DOC.contains(&with_token), "docs/mcp.md doesn't show `{with_token}`");
}
