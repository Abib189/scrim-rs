# R5e: Describe a change, verified (22 Oct 2026)

**The screen was locked all night,** and a locked Mac draws no window. So nothing here is a capture of the real window, and the composer couldn't be driven by clicks in it. Instead:

## 1. The stub agent, end to end, headless (`apps/figcraft/src/chat_e2e_tests.rs`)

**What ran:**
- **The window's own code**, frame by frame on the wall clock, as the app runs it: the requests that arrived, `logic`, then the UI.
- **The app's real MCP endpoint** (`McpHandle`) on a free port: never 7680, and the logs say so.
- **The real runner** (`agent_runs` over `figcraft_mcp::agent_run`).
- **A real child process:** `crates/mcp/tests/stub/stub_agent.py`, which reads Claude Code's real arguments and calls the endpoint over HTTP.
- **The person's part:** the send was the person's `chat.send`, after `chat.allow-agent`.
- **No settings were saved:** these services keep none. The stub's `HOME` was a scratch folder.

| Run | What happened | Log |
|---|---|---|
| Send "batches 3 slow 2" | Running at once. Three batches of 3 rectangles landed through MCP. The dots came on and the new layers had their entrance; following never paused. The transcript reads: you; "Agent: made 3 layers" ×3; the reply "Made batch 1, batch 2, batch 3."; "0:03 · Billed by your agent's plan." The dots were gone when it ended (the stub called `finish_working`). Diagnostics: "Claude Code run started", begin, end, "Claude Code run ended: finished, exit 0, 4 s". | `logs/stub-run.json` |
| Stop mid-run | Two batches landed, then Stop. The stub's whole process group (it and its own child, `sleep 300`) was gone 0.04 s after Stop; the dots ended at once ("end: … finish_working"). "Stopped. What the agent made so far stays: Undo takes it back." The 6 rectangles stayed; two Undo steps removed them. | `logs/stub-stop.json` |
| The time limit (3 s, `SCRIM_CHAT_LIMIT_SECS`) | Stopped at 3.07 s: "Stopped after 3 s, this run's time limit." The rectangle it made stayed. Diagnostics: "timed out". | `logs/stub-time-limit.json` |

Run them with `cargo test -p figcraft --bin figcraft stub_ -- --ignored --test-threads=1`, adding `SCRIM_CHAT_LIVE_OUT=<dir>` for the logs.

## 2. One real Claude Code run (`real_claude_once`, `SCRIM_LIVE_CLAUDE=1`)

- **The setup:** once, a 2-minute cap, "Create one 100x100 rectangle named Test". Scrim ran in-process on a free port with no settings saved. Only the CLI got the real `HOME`, for its own sign-in.
- **What happened:** Claude Code 2.1.296 accepted every argument (no usage error) and answered in `stream-json`. Its result event was an error, "Not logged in · Please run /login", exit 1 after 1.4 s, reporting 0 tokens and $0.00. Nothing was made.
- **NOT verified: a real agent making a change.** As the brief says, the run wasn't retried and the sign-in wasn't touched.
- **Why it wasn't signed in:** not known. Two likely causes:
  - the run's environment is only `PATH`, `HOME` and `SCRIM_CHAT_CHILD` (LOCKED decision 4), and Claude Code's sign-in on macOS may need more of it, `USER` for one;
  - or its keychain item can't be read while the screen is locked.
- **Found by this run, and fixed:** the CLI's error came back as its result, so the transcript showed it as a reply, and the failure line said "no message". An error result is now the failure's words: "Claude Code stopped before it began: Not logged in · Please run /login". `chat_ui_tests.rs` pins it with the event as the real CLI sent it.
- **The log:** `logs/real-claude.json`.

## 3. Captures, light and dark (offscreen)

Drawn by the offscreen renderer (`offscreen_tests.rs` › `chat_screenshots`, the software rasteriser kept for a locked screen). That is the window's own drawing filled in software, not a window capture. The document is `examples/ridgeline-app.figcraft`, with a scripted run in place of the runner:

| File | State |
|---|---|
| `chat-{light,dark}-1-first-use-asks.png` | "Run Claude Code?": what it does, "Program: …", the whole command with `<your message>` |
| `chat-{light,dark}-2-running.png` | Two batches in: "Agent: made 3 layers" ×2, Stop, "Claude Code is working · 0:12" |
| `chat-{light,dark}-3-finished.png` | The reply and "0:38 · Billed by your agent's plan."; Send disabled, "Type what to change" |
| `chat-{light,dark}-4-settings.png` | Settings ▸ AI & agents › Describe a change |
| `chat-{light,dark}-5-no-agent-installed.png` | The empty state with the two docs buttons |

- **The capture found a bug:** the confirmation showed `--tools  --allowedTools`, dropping the empty argument. `command_line` now quotes as a shell needs (`--tools ''`), for Connect's commands too.
- **Personal data:** every capture was read with `scripts/ocr-captures.swift`, and the logs searched. There's no user name, home folder or token; the program path in the captures is the test's `/opt/agents/bin/claude`.

## Not verified

- **The real window:** clicks, keys and pixels; the screen was locked.
- **A real agent making a change:** Claude Code wasn't signed in within the run (above).
- **Codex, live.**
- **Windows and Linux runs:** the runner's Unix process-group kill is tested on macOS here; Windows' `taskkill /T /F` isn't run anywhere.
- **Long runs:** nothing over 5 s, except the time-limit logic itself.
- **Authentication on:** the `${SCRIM_MCP_AUTH}` header expansion inside `--mcp-config` (Claude Code's documented behaviour, not tried).
