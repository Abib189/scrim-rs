# Describe a change: the AI tab's composer (R5)

The design: `docs/design/chat-composer.md`.

| Check | How | Covered by |
|---|---|---|
| R5a: Claude Code and Codex are headless profiles (data); a run's arguments are the template filled in, one argument each, the prompt never split or interpreted, the token never in an argument | Read `headless.json`; the confirmation shows the command | `crates/engine/src/cmd/agent_clients/headless_tests.rs` |
| R5a: an agent run gets only `PATH`, `HOME`, the token (authentication on) and `SCRIM_CHAT_CHILD=1`, in a new empty folder removed afterwards | Run the stub with `env` / `cwd` in its prompt | `crates/mcp/tests/agent_run.rs` › `the_child_sees_only_its_minimal_environment`, `the_run_happens_in_an_empty_folder_that_is_removed` |
| R5a: Stop, the time limit and closing Scrim end the agent and everything it started | Stop a slow run; `ps` shows nothing left | `agent_run.rs` › `stop_ends_the_whole_process_tree`, `the_time_limit_stops_it`, `closing_scrim_mid_run_leaves_nothing_running` |
| R5a: the reply, and tokens or cost only when the CLI reports them; a failing start's last error line with tokens redacted; a non-zero exit; a missing binary; an unreachable Scrim | The stub's `cost`, `login`, `exit3`, `unreachable` prompts | `agent_run.rs`; `headless_tests.rs` › `error_output_is_shown_with_tokens_redacted`, the parser tests |
