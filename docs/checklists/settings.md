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
