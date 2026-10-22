# Fix S5 live run: the bridge across Scrim restarts

`s5.py BINARY SCRATCH OUT` runs the app on a scratch HOME and a scratch settings folder (`SCRIM_SETTINGS_DIR`), with MCP on 7690, then 7691. It runs `BINARY mcp-bridge` on the same folders, with this script as its client on stdin and stdout.

- `s5.json`: every event, timed from the start: each call with how long it took and the start of its answer, each `notifications/tools/list_changed`, and each app start and quit.
- `bridge-stderr.txt`: the bridge's own log.
- `s5-scrim.json` and `s5-scrim-bridge-stderr.txt`: the same run with the `scrim` CLI's bridge (`s5.py … target/debug/scrim`), the binary the brief names. The app's own binary (`figcraft mcp-bridge`) is the one Connect writes into Claude Desktop's entry.

| Bridge | Call in flight at each quit | `list_changed` (same port, another, quiet) | Next call after each restart | Slowest of all calls |
|---|---|---|---|---|
| `figcraft mcp-bridge` (`s5.json`) | "Scrim is not running…", 2 ms | yes, yes, yes | answered ×3 | 38 ms of 67 |
| `scrim mcp-bridge` (`s5-scrim.json`) | "Scrim is not running…", 1–2 ms | yes, yes, yes (1.5 s after the quiet restart) | answered ×3 | 29 ms of 66 |

The run kept here is the one after the fix. Before it, the third restart (same port, no call while down) sent no `list_changed`, and the bridge didn't connect again. Summary and decisions: `docs/design/connect-claude-desktop.md` › Fix S5.

**Debug build, under load.** The times are from this run only; no timings are claimed.
