# Fix S5 live run: the bridge across Scrim restarts

`s5.py BINARY SCRATCH OUT` runs the app on a scratch HOME and a scratch settings folder (`SCRIM_SETTINGS_DIR`), with MCP on 7690, then 7691. It runs `BINARY mcp-bridge` on the same folders, with this script as its client on stdin and stdout.

- `s5.json`: every event, timed from the start: each call with how long it took and the start of its answer, each `notifications/tools/list_changed`, and each app start and quit.
- `bridge-stderr.txt`: the bridge's own log.

The run kept here is the one after the fix. Before it, the third restart (same port, no call while down) sent no `list_changed`, and the bridge didn't connect again. Summary and decisions: `docs/design/connect-claude-desktop.md` › Fix S5.

**Debug build, under load.** The times are from this run only; no timings are claimed.
