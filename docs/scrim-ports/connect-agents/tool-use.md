# Tool use, live (Stage K)

The record behind each client profile whose `tool-use` is `verified-live` (`crates/engine/src/cmd/agent_clients/clients.json`). A profile's `tool-use` becomes `verified-live` only with an entry here: the client, the date, and what happened. A test (`agent_clients_tests.rs`) checks that every such profile points at a doc that names it and its date.

**Registration is separate.** Adding Scrim to a client (its `registration` level) is recorded with each client's own run: Claude Code in `docs/scrim-ports/settings/README.md` (the real Connect, captures 12–14), Codex in `README.md` beside this page.

## Claude Code, 2026-10-21 (21 Oct 2026)

- **What happened:** during the Stage K session, Claude Code called Scrim's `inspect_ui` tool on the owner's running Scrim. It reached the window through the MCP endpoint over HTTP, and the window answered with its state:
  - the follow state, the canvas, the status;
  - the window list: one window, "Untitled – Scrim".
- **Observed by:** the agent itself, in the session that made this change. The call was read-only and changed nothing.
- **Not recorded:** the open file's name and path, which are the owner's.
- **Earlier work:** Claude Code also used Scrim's tools during Stages C to J (the C4 live proofs and the owner's own sessions), but none of that was recorded as a tool-use session.

## Claude Desktop, 2026-10-21 (21 Oct 2026)

- **The record:** Claude Desktop, 21 Oct 2026, roughly 00:12 to 12:44 local time, first-hand by the agent in the chat: used Scrim's tools through the `scrim` server in Claude Desktop: inspect_document, batch, set_properties, screenshot, set_lint_profile, lint_design, ignore_lint, run_command (edit.duplicate, file.export, file.saveAs). Built two posters of about 100 layers in total and wrote PDF, PNG and .figcraft exports. The agent observed the scrim entry appear in Desktop's server list between 00:06 and 00:08; how that entry was written (Scrim's Connect Claude Desktop button or by hand) is unknown.
- **How it reached this page:** the owner passed on the Desktop agent's own account in the Fix L brief (it replaces Stage K's entry, which rested on the owner's report alone). No file names, paths or document content are recorded. No capture or log of the session is in this repository.
- **Its registration stays docs-only:** how the `scrim` entry was written is unknown, and no live run of Connect Claude Desktop against Claude Desktop's own configuration is recorded. Stage F checked the bridge with a scratch configuration, not Claude Desktop itself. It changes only if the owner confirms Scrim's button wrote the entry.
