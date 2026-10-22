# Describe a change: the AI tab's composer (R5)

**Status:** design, 22 Oct 2026, before any code. The decisions marked **LOCKED** were made in the brief; this page records them and how they're built. Everything else is a reading of the brief, listed at the end for the owner to overrule.

## What it is

At the bottom of the AI tab, a field: "Describe a change…". The person types what they want, picks an agent, and presses Send. Scrim runs that agent's command-line program (Claude Code or Codex, already installed and signed in by the person) as a child process, pointed at Scrim's own MCP endpoint. The agent then works on the canvas through MCP like any connected agent: working dots, follow, reveal and `finish_working` all apply unchanged. Its final reply appears in a transcript above the field.

## The LOCKED decisions

1. **No model of its own, no keys.** Scrim stores no API key and calls no model. It runs a local agent CLI the person already has, as a child process, and gives it Scrim's MCP endpoint.
2. **Headless profiles, as data.** Claude Code (`claude -p`) and Codex (`codex exec`) first, each a profile next to the 16 client profiles: name, how to find the binary, the argument template, the output parser, the docs URL (`crates/engine/src/cmd/agent_clients/headless.json`).
3. **Each run passes its own MCP configuration**, naming the endpoint the window is on now (after a port fallback too, and in a scratch test never the person's own Scrim). The CLI's strict mode keeps every other MCP server out of sight.
4. **A minimal environment:** `PATH`, `HOME` (for the CLI's own login), the bearer token when authentication is on, and `SCRIM_CHAT_CHILD=1`. The child is spawned from an explicit argument list, never a shell, from a binary path resolved beforehand and shown.
5. **Nothing new is granted.** The child is one more MCP client, held to Settings ▸ Tool access (the Read and Write classes of `command-access.md`).
6. **The chat commands are the person's alone.** `chat.send`, `chat.cancel` and `chat.clear` are refused for agents on every route (MCP, the control channel, headless): an agent can't start agents. A Scrim started by an agent run (`SCRIM_CHAT_CHILD` in its environment) refuses `chat.send` too. They're classified in `command-access.md`, which stays complete.
7. **First use of each agent asks once,** showing the exact command line and binary path, as Connect does. The answer is remembered per profile and binary path; a different path asks again.
8. **What is sent:** the typed prompt, plus one context line (the page's name, and the selected layers' ids and names). Nothing else. The agent reads the document only through MCP, under Tool access.
9. **Stopping:**
   - **Stop and Esc** kill the child's whole process tree, end its working state the way `finish_working` does, and say "Stopped".
   - **Partial edits stay** as ordinary "Agent: …" undo steps, so ⌘Z takes them back.
   - **A time cap** stops a run after 15 minutes. Settings ▸ AI & agents offers 5, 15, 30 or 60.
   - **One run at a time per window:** Send is disabled while one runs, and says why.
10. **Cost:** Scrim doesn't know it. It shows the elapsed time, and tokens or cost only when the CLI reports them in its final output; otherwise the fixed line "Billed by your agent's plan." It never estimates.
11. **Privacy:**
    - **The transcript lives in memory** for the window's life; nothing is saved by default.
    - **Settings ▸ Diagnostics logs only** the agent's name, when a run started and ended, its exit status and its duration. Never the prompt or the output.
    - **A Settings switch, "Describe a change"** (on by default), turns the whole composer off.
12. **The UI:**
    - **The composer** sits at the bottom of the AI tab: a multiline field (⌘↩ sends), the agent picker, and Send or Stop.
    - **The picker** lists only the installed agents that have a headless profile, Claude Code first. With none installed, it says so, with links to the install docs.
    - **The transcript** sits above the field: your messages, the agent's final reply, and one quiet line per batch of work ("Agent: made 12 layers").
    - **Look and access:** Scrim's tokens and accent helpers (no hard-coded orange), reachable by keyboard, with AccessKit names, in both themes, with text at 4.5:1 and other marks at 3:1.
13. **Each failure has plain words and a test:** the CLI not found; not signed in, or failing at once (its last error line shown, tokens redacted); a non-zero exit; the time cap; the MCP endpoint down; Scrim closed mid-run.

## The flags, as this machine's CLIs document them

**Claude Code 2.1.296** (`claude --help`, read 22 Oct 2026):

```
claude -p <prompt + context line>
  --output-format stream-json --verbose     one JSON event per line; the last is the result
  --mcp-config <JSON>                       this run's servers: only Scrim's
  --strict-mcp-config                       "Only use MCP servers from --mcp-config"
  --tools ""                                "Use "" to disable all tools" (built-in: no shell, no file edits)
  --allowedTools mcp__scrim                 Scrim's tools may run without asking
  --permission-mode dontAsk                 anything not allowed is refused, never asked
  --no-session-persistence                  the session isn't saved to disk
```

- **The MCP configuration:** `{"mcpServers": {"scrim": {"type": "http", "url": "http://127.0.0.1:<port>/mcp"}}}`. With authentication on, it adds `"headers": {"Authorization": "Bearer ${SCRIM_MCP_AUTH}"}`, and the token itself is only in the child's environment, never in its arguments.
- **What the help text doesn't state:** that `mcp__scrim` matches all of a server's tools, what `dontAsk` refuses, and that `${VAR}` is expanded inside `--mcp-config`. These come from Claude Code's permissions and MCP documentation. The live run (R5e) has authentication off.
- **`--bare` is not used:** it reads no OAuth login or keychain, only an API key, and the person signs in with their plan.

**Codex CLI 0.162.1** (`codex exec --help`):

```
codex exec <prompt + context line>
  --json                                    events as JSON lines
  --ephemeral                               no session files on disk
  --skip-git-repo-check                     it runs in an empty folder
  --ignore-user-config                      ~/.codex/config.toml isn't loaded: only Scrim's server
  -s read-only                              its shell can't write
  -c mcp_servers.scrim.url="http://127.0.0.1:<port>/mcp"
  -c approval_policy="never"
  [-c mcp_servers.scrim.bearer_token_env_var="SCRIM_MCP_AUTH"]    with authentication on
```

- **Codex has no `--mcp-config` flag.** The closest documented route is a `-c` override of `mcp_servers.<name>`, with `--ignore-user-config` keeping the person's other servers out ("auth still uses `CODEX_HOME`", so the login works).
- **The key names come from Codex's configuration reference:** `url`, `bearer_token_env_var` and `approval_policy`. This binary's help doesn't list them. Codex isn't verified live (below).

## How a run goes

1. **Send** (`chat.send {prompt, agent?}`, the person only) asks the window to run.
2. **The window checks, in order, and says plainly when one fails:**
   - the composer is on;
   - no run is under way;
   - the MCP endpoint is running (else nothing starts);
   - the agent's binary is found (`Services::find_program`: `PATH`, then the usual install folders);
   - this profile and path were confirmed (else the confirmation opens; Run confirms and sends).
3. **The runner** (`crates/mcp/src/agent_run.rs`, desktop only) starts the binary:
   - with the profile's arguments, filled in;
   - in a new empty folder, so the agent sees none of the person's files; it is removed afterwards;
   - with the minimal environment;
   - in a process group of its own (Unix), with stdin closed.
   - It reads stdout line by line through the profile's parser (`agent_output.rs`, in the engine: pure, tested) and keeps the last lines of stderr.
4. **Meanwhile the window** shows the elapsed time. Each batch the agent lands adds a line ("Agent: made 12 layers") from the change log. The dots, follow and reveal work as for any agent: the child announces its own `clientInfo` (`claude-code`, `codex-mcp-client`).
5. **When it ends:**
   - the reply, or the failure in plain words;
   - the tokens or cost if the CLI reported them, else "Billed by your agent's plan.";
   - the child's MCP sessions are ended as `finish_working` ends them, so the dots stop at once;
   - Diagnostics gets one line with no content.
6. **Stop, Esc, the time cap and closing Scrim** all kill the process group: `SIGTERM`, then `SIGKILL` after 2 s (Unix, through `kill`, as an argument list); `taskkill /T /F` on Windows.

## The parsers

- **Claude Code** (`stream-json`): the `system`/`init` event lists the MCP servers and their status, so a Scrim that couldn't be reached shows as such. `assistant` events carry text and tool calls. The final `result` event has `result` (the reply), `is_error`, `total_cost_usd`, `usage` and `duration_ms`.
- **Codex** (`--json`): `item.completed` with an `agent_message` item is the reply; `turn.completed` carries `usage`; `turn.failed` and `error` are failures.
- **Anything unknown** is skipped. A line that isn't JSON is ignored, not shown.

## Settings (`Settings ▸ AI & agents`)

- **"Describe a change"** (`describe_change`, on): off hides the composer and refuses `chat.send`.
- **"Stop a run after"** 5, 15 (default), 30 or 60 minutes (`chat_time_limit_mins`); another saved value goes to the nearest.
- **The confirmed agents** (`chat_allowed`: profile and binary path), with **Forget** to ask again.
- **Each switch is a command** (`pref.describeChange`, `pref.chatTimeLimit`, `chat.allow-agent`, `chat.forget-agents`), and the person's alone, like the chat commands.

## Tests

| Part | Tests |
|---|---|
| **R5a** profiles, parser, runner | Profiles load and fill their templates (no shell, the token never in the arguments). Each parser on recorded event shapes. The runner with a **stub agent** (`crates/mcp/tests/stub_agent.py`): it reads the same arguments, calls a real Scrim MCP endpoint over HTTP and prints Claude-style events. It covers the reply; the stop killing a grandchild; the time cap; a failing start's last stderr line, redacted; a non-zero exit; a missing binary; and the minimal environment. |
| **R5b** commands | Each chat command, run as an agent over MCP, the control channel and headless, is refused with its words, and every command is still classified. With `SCRIM_CHAT_CHILD` set, `chat.send` is refused. |
| **R5c** UI | The composer and transcript in the window harness: Send disabled with a reason; ⌘↩; Stop and Esc; the empty state; the batch lines; both themes' contrast; AccessKit names. |
| **R5d** settings, Diagnostics, confirmation | The switch off hides it all. The time limit's values. The first use asks; a changed path asks again. Diagnostics lines hold no prompt or output. |
| **R5e** live | The stub through the real window; one real Claude Code run (below). |

## Live verification (R5e)

- **Everything on a scratch settings folder and a scratch `HOME`,** MCP on 7690, never 7680.
- **The stub** through the real window:
  - send, dots, follow;
  - Stop mid-run, then undo of the partial work;
  - a short test cap.
- **One real `claude` run, once:** "Create one 100x100 rectangle named Test", at most 2 minutes.
  - Only the CLI gets the person's real `HOME`, for its own login. `SCRIM_CHAT_CHILD_HOME` names the `HOME` given to agent runs; it exists for this check and defaults to the app's own `HOME`.
  - If the login or flags fail, it's recorded as not verified, and nothing is retried or changed in the login.
- **Captures** in light and dark under `docs/scrim-ports/chat/`, OCR-checked under R1's rules. If the screen is locked, they come from the offscreen renderer, and the README says so.

## Readings of the brief (overrule any)

1. **The runner lives in the MCP crate,** desktop only. The engine holds the profiles, parsers and redaction (pure, wasm-safe), and the window reaches the runner through `Services`. The browser build has no composer: it can't start programs.
2. **Agents can't change these settings either:** `pref.describeChange`, `pref.chatTimeLimit` and the confirmation commands are the person's alone, like the chat commands.
3. **A run's working state is ended by the child's client name** (and the sessions that announced it after the run began), since Scrim doesn't see the child's MCP session id before its first request.
4. **An empty working folder,** removed after the run, so the agent can't read or write the person's files. For Codex that also means `-s read-only`.
5. **The batch lines count every agent edit logged while the run is under way.** If another agent edits at the same time, its batches are counted too.
6. **`--verbose`** is passed with `stream-json`. Claude Code needs it in print mode, from its documentation; that wasn't tried without it, so as not to spend a run.
7. **The environment adds the variables Windows can't start a program without** (`SYSTEMROOT`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `TEMP`). On macOS and Linux it is exactly the four of decision 4.
8. **Closing the window with a run under way** kills it without asking. The transcript goes with the window, as decided.

## R5a as built: profiles, parsers and the runner

- **The profiles:** `crates/engine/src/cmd/agent_clients/headless.json` holds Claude Code and Codex with the flags above, each with the `clientInfo` name it announces and where its flags were read.
- **Arguments:** `Headless::argv` fills the template one argument at a time. `{auth_args}` stands alone, as zero or more arguments; nothing is split or quoted. A prompt full of shell syntax stays one unchanged argument.
- **The confirmation's text:** `Headless::display` shows the binary path and every argument, with `<your message>` for the prompt.
- **The engine also has:**
  - `child_env` (the minimal environment; Windows adds its system variables);
  - `context_line` and `message` (the prompt plus the one line);
  - `redact` (the token itself, bearer values, `sk-`/GitHub key shapes and `key=`/`token:` values become `<token>`; 300 characters at most);
  - `output::parse` and `Outcome` (both parsers; tokens and cost only as reported).
- **The runner** (`crates/mcp/src/agent_run.rs`):
  - **`Run::start`** clears the environment and passes only what it's given. It runs in a new empty folder (removed afterwards), with stdin closed, in a process group of its own.
  - **Output:** stdout comes back as lines; the last 20 lines of stderr are kept.
  - **The end:** `RunEvent::Ended` says how it ended (exited with a code, stopped, timed out) and how long it took.
  - **Stopping:** `Run::stop`, the limit, and dropping the `Run` all signal the group: `kill -TERM -- -<pgid>`, then `-KILL` after 2 s (`kill`, as an argument list); `taskkill /T /F` on Windows.
- **The stub agent** (`crates/mcp/tests/stub/stub_agent.py`): it takes Claude Code's real arguments, reads the MCP configuration, and calls the endpoint over HTTP (initialize, tools, `finish_working`). It prints Claude Code's events, and its prompt's words pick a failure.
- **Tests:**
  - `headless_tests.rs` (12): profiles, arguments, the token never in an argument, a hostile prompt, the environment, the context line, redaction, both parsers, nothing estimated.
  - `crates/mcp/tests/agent_run.rs` (11, Unix):
    - the reply and the rectangle in Scrim's document;
    - reported usage and cost;
    - Stop ends the agent and its own child within 5 s;
    - the time limit;
    - dropping the run (Scrim closing) leaves nothing running;
    - a sign-in failure's last line, redacted;
    - exit 3;
    - an unreachable Scrim;
    - a missing binary;
    - only the minimal environment;
    - the empty folder, removed.
  - **Mutation:** without the process group, the three stop tests fail.

## R5b as built: the commands, the person's alone

- **`crates/engine/src/cmd/chat.rs`** registers seven commands, none of them an undo step:
  - `chat.send {prompt, agent?}` (Claude Code unless named; the prompt trimmed, at most 20,000 characters);
  - `chat.cancel`, `chat.clear`;
  - `chat.allow-agent {agent, path}` (the first-use confirmation: remembered, and the waiting send goes on);
  - `chat.forget-agents`;
  - `pref.describeChange {on}`;
  - `pref.chatTimeLimit {minutes: 5|15|30|60}`.
- **What they do:** they only ask the window (`chat.send`, `chat.cancel`, `chat.clear`, `chat.allowed` requests) or change their settings. Send, stop and clear add no revision, like `agent.finish-working`.
- **Refused for agents on every route,** with one sentence: "Describe a change is the person's: agents can't start, stop or set up agent runs". The check is the command's own (the author of the request), so MCP's `run_command` and `batch`, the control channel, the headless CLI and plugins all meet it.
- **Started by an agent run:** the app reads `SCRIM_CHAT_CHILD` at startup (`Agents::chat_child`). Such a Scrim refuses `chat.send` even from the person: "this Scrim was started by an agent run, so it can't start another".
- **The settings:**
  - `describe_change` (on), and `chat_time_limit_mins` (15; another value goes to the nearest of 5, 15, 30, 60);
  - `chat_allowed` (agent and path, empty pairs dropped, no duplicates);
  - all with serde defaults, so no version bump.
- **`command-access.md`** lists all seven under Write, and under a new "The person's alone" section. It was behind: 20 commands from Stages I–K and Fix Q were missing (`camera.*`, the Connect commands, `pref.agentGrace`, `pref.revealSpeed`…). They're added. A new test, `access_tests.rs` › `this_page_lists_every_classified_command`, fails when a classified command is missing or a heading's count is wrong.
- **Tests:**
  - `chat_tests.rs` (13):
    - every command refused for agents on five routes, with no setting changed and the window not asked;
    - a batch can't carry one;
    - the person's send asks the window;
    - an empty, unknown or too long prompt;
    - off, or started by an agent run;
    - the time limits;
    - allowing per binary, and forgetting;
    - every command classified Write;
    - the environment check.
  - `crates/mcp/tests/chat_refused.rs`: over MCP, `run_command` and `batch`.
  - `chat_route_tests.rs`: the control channel and the app's MCP route.
