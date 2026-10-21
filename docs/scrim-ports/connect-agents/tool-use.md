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

- **What happened:** an agent called Scrim's tools through Claude Desktop on 21 Oct 2026.
- **Source:** **the owner's report**, recorded at their request (the Stage K brief). It wasn't observed in a Scrim session here: no capture or log of it exists in this repository.
- **What that leaves unverified:**
  - which tools were called;
  - how Claude Desktop was registered (by Connect Claude Desktop, or by hand);
  - whether it went through the `mcp-remote` bridge.
- **Its registration stays docs-only:** no live run of Connect Claude Desktop against Claude Desktop's own configuration is recorded. Stage F checked the bridge with a scratch configuration, not Claude Desktop itself.
