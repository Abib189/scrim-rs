//! The copyable connect commands the app offers (`figcraft_engine::agents::connect_command`) are
//! the ones `docs/mcp.md` shows, and the server announces itself as `scrim` (A3).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use figcraft_engine::agents::connect_command;

const MCP_DOC: &str = include_str!("../../../docs/mcp.md");

#[test]
fn the_apps_connect_commands_are_the_documented_ones() {
    for cmd in [connect_command(None), connect_command(Some(7979))] {
        assert!(MCP_DOC.contains(&cmd), "docs/mcp.md doesn't show `{cmd}`");
        assert!(cmd.contains("scrim mcp") && !cmd.contains("figcraft"), "{cmd}");
    }
    assert!(!MCP_DOC.contains("claude mcp add figcraft"), "docs/mcp.md still shows the old command");
}
