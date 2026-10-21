# Connect any agent (Stage J)

The plan and the clients' documentation: `docs/design/connect-any-agent.md`.

| Check | How | Covered by |
|---|---|---|
| J1: a client's name is its friendly name when known, as announced otherwise (cut at 32 with "…"), "an agent" when it gave none; never "Claude" for another client | Connect Codex, then an unknown client | `crates/engine/src/agents_tests.rs` › `client_names_are_friendly_when_known_verbatim_otherwise_and_never_guessed` |
| J1: each MCP request is attributed to its session's client (`Mcp-Session-Id` on `initialize`); without one, the newest client | Two clients at once | `crates/mcp/tests/sessions.rs` |
| J1: an agent's edit records its client in the change log (`history.changes` › `client`); the person's never | — | `agents_tests.rs` › `the_change_log_names_the_client_of_an_agents_edit` |
| J1: "Connected · …", "Following … · Poster", "… is working on Poster" (AccessKit), the Activity row and diagnostics name the client, for Claude Code, Codex, Gemini CLI, Cursor, an unknown, an empty and a very long name; follow, the reveal and the dots behave the same for each | Connect each client, let it edit | `crates/ui-egui/src/client_names_tests.rs` › `every_place_names_the_client_and_behaves_the_same_for_each` |
| J1: with several clients, all are listed and the latest to edit is named | Codex and Gemini CLI at once | `with_several_clients_the_latest_to_edit_is_named_and_all_are_listed` |
