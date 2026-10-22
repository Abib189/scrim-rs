# Describe a change: the AI tab's composer (R5)

The design: `docs/design/chat-composer.md`.

| Check | How | Covered by |
|---|---|---|
| R5a: Claude Code and Codex are headless profiles (data); a run's arguments are the template filled in, one argument each, the prompt never split or interpreted, the token never in an argument | Read `headless.json`; the confirmation shows the command | `crates/engine/src/cmd/agent_clients/headless_tests.rs` |
| R5a: an agent run gets only `PATH`, `HOME`, the token (authentication on) and `SCRIM_CHAT_CHILD=1`, in a new empty folder removed afterwards | Run the stub with `env` / `cwd` in its prompt | `crates/mcp/tests/agent_run.rs` › `the_child_sees_only_its_minimal_environment`, `the_run_happens_in_an_empty_folder_that_is_removed` |
| R5a: Stop, the time limit and closing Scrim end the agent and everything it started | Stop a slow run; `ps` shows nothing left | `agent_run.rs` › `stop_ends_the_whole_process_tree`, `the_time_limit_stops_it`, `closing_scrim_mid_run_leaves_nothing_running` |
| R5a: the reply, and tokens or cost only when the CLI reports them; a failing start's last error line with tokens redacted; a non-zero exit; a missing binary; an unreachable Scrim | The stub's `cost`, `login`, `exit3`, `unreachable` prompts | `agent_run.rs`; `headless_tests.rs` › `error_output_is_shown_with_tokens_redacted`, the parser tests |
| R5b: `chat.send`, `chat.cancel`, `chat.clear`, `chat.allow-agent`, `chat.forget-agents`, `pref.describeChange` and `pref.chatTimeLimit` are refused for agents on every route (MCP, control channel, headless, plugins), with "Describe a change is the person's…"; a Scrim started with `SCRIM_CHAT_CHILD` refuses `chat.send` | Over MCP, `run_command chat.send`; start Scrim with `SCRIM_CHAT_CHILD=1` and press Send | `cmd/chat_tests.rs`; `crates/mcp/tests/chat_refused.rs`; `chat_route_tests.rs` |
| R5b: `docs/design/command-access.md` lists every classified command | Read it | `access_tests.rs` › `this_page_lists_every_classified_command` |
| R5c: the composer at the bottom of the AI tab (picker of installed agents, field, Send/Stop, Clear, status), the transcript above it; ⌘↩ sends, Esc stops; Send disabled with its reason | Open the AI tab, type, ⌘↩; Stop mid-run; Undo | `chat_ui_tests.rs` |
| R5c: the first run of an agent from a program asks, showing the program and the whole command; another program asks again | Send for the first time; move the CLI | `chat_ui_tests.rs` › `the_first_use_asks_with_the_program_and_the_command_then_runs`, `another_program_asks_again` |
| R5c: the transcript says the work ("Agent: made 3 layers"), the reply, and "Billed by your agent's plan." unless the agent reported tokens or cost; each failure in plain words | Run the stub with `cost`, `login`, `exit3`, `unreachable` | `chat_ui_tests.rs` › `a_run_reads_…`, `tokens_and_cost_…`, `each_failure_is_said_in_plain_words` |
| R5c: its text reads at 4.5:1 and the field's edge at 3:1 in both themes | Look in light and dark | `chat_ui_tests.rs` › `its_text_reads_at_4_5_to_1_and_its_field_edge_at_3_to_1_in_both_themes` |
