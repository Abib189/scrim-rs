# Settings, one-click connect, the AI tab's Connect (Stage C)

The plan: `docs/design/settings-and-connect.md`.

| Check | How | Covered by |
|---|---|---|
| The app serves MCP over HTTP on 127.0.0.1:7600 (or the next free port, kept) from launch | Start the app; `curl -s -X POST -H 'Host: 127.0.0.1:7600' -d '{"jsonrpc":"2.0","id":1,"method":"ping"}' http://127.0.0.1:7600/mcp` | `crates/mcp/tests/http.rs` › `the_mcp_round_trip_over_http`, `a_taken_port_falls_back_to_the_next_free_one`; `mcp_endpoint_tests.rs` |
| Requests from another Host (DNS rebinding) or a web page's Origin are refused with 403, before anything runs | `curl -H 'Origin: http://evil.example' …` | `requests_from_elsewhere_are_refused`; `refused_requests_never_reach_the_ui_and_a_token_applies_at_once` |
| Require authentication: a token is made once; requests without it get 401; the connect command carries it | Settings ▸ MCP, then curl with and without the header | `a_required_token_must_be_sent`; `authentication_makes_a_token_once_and_the_command_carries_it` |
| Tool calls over HTTP run as the agent "mcp (http)": locks bind them, their undo steps say "Agent: …" | Edit over HTTP, then look at Edit ▸ Undo and the AI tab | `tool_calls_reach_the_ui_as_the_mcp_agent` |
| File tools over HTTP are confined to the root folder (home by default) | Settings ▸ MCP ▸ Choose folder, then `open_file` outside it | `control.rs` › `handle_request` (manual) |
| `scrim mcp` (stdio, headless or `--attach`) still works | `claude mcp add scrim -- scrim mcp` | `crates/mcp/tests/protocol.rs` |
| Restart keeps the port | Settings ▸ MCP ▸ Restart | `a_restart_listens_on_the_same_port_again` |
| AI tab, not connected: a status row (Not connected / Server stopped with Restart) and Connect Claude Code; connected: "Connected · <client>" | Open the AI tab before and after connecting | `ai_tab_tests.rs` › `the_status_row_says_stopped_not_connected_or_connected` |
| Connect shows the exact command with Run and Copy; Run asks once; nothing runs before; then `claude mcp get` checks, and `claude mcp add` runs with an argument list | Connect Claude Code, Run, Run | `connect_flow_tests.rs` › `connect_shows_the_command_runs_nothing_until_confirmed_then_adds_scrim` |
| A `scrim` entry already there is offered for Replace, never overwritten; an old `figcraft` entry can be removed (confirmed) | With an existing entry, Connect | `an_entry_already_there_is_offered_for_replace_never_overwritten` |
| Without `claude`: Copy only, and why | On a machine without Claude Code | `without_claude_only_copy_is_offered` |
| An agent can't make Scrim run a program | `run_command {command: "mcp.run-connect"}` over MCP | `an_agent_cannot_make_scrim_run_a_program` |
| The AI tab shows no "--control" or "scrim mcp" | — | `the_ai_tab_shows_no_control_channel_or_scrim_mcp_copy` |
| Settings opens from the home sidebar's bottom row, Cmd+, and the palette (`settings.open`); Done and Escape close it; an agent's `settings.open` opens nothing and returns the settings | Click Settings at the bottom of Home; press Cmd+, in a file | `settings_tests.rs` › `settings_opens_from_its_command_cmd_comma_and_closes_with_done_or_escape`, `an_agents_settings_open_returns_the_settings_and_opens_nothing` |
| Settings ▸ General ▸ Theme and the tab bar's sun/moon are one setting (`pref.theme`): each follows the other | Toggle the moon, open Settings; choose Light there | `the_theme_in_settings_and_the_tab_bars_toggle_follow_each_other` |
| Animations: Follow system / On / Off | Choose Off: hovers and openings don't animate | `animations_follow_system_on_or_off` |
| Keep recovery copies of unsaved work (on by default); off stops new copies | Turn it off, then force-quit with an unsaved change | `keeping_unsaved_work_is_a_kept_setting` (the setting); the recovery tick's guard in `apps/figcraft/src/main.rs` (manual) |
| The home sidebar's Settings row: pinned at the bottom below a hairline, the same row as the others, which stay where they were | Look at Home in a short and a tall window | `the_home_sidebar_has_settings_pinned_at_the_bottom` |
