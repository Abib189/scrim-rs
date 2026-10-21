# Fix P3: Claude Desktop's entry, started before Scrim (22 Oct 2026)

**Claude Desktop itself was not driven.** A script (`p3.py`) started the entry Connect Claude Desktop builds, exactly as Desktop starts a local server: its command and arguments, with stdin and stdout as the MCP channel. It then spoke to it as Desktop does: `initialize` as "claude-ai", `notifications/initialized`, `tools/list`, `tools/call`.

**The setup:**
- **The app:** the release build of `1d19889`, on a fresh scratch `HOME` per run. Its Settings ▸ MCP port was 7690, so nothing here could reach the real Scrim on 7680, which was running.
- **The entry,** read from that app (`mcp.claude-desktop-status`): `{"command": "<repo>/target/release/figcraft", "args": ["mcp-bridge"]}`. It wasn't written into a Claude Desktop file: the runs started it directly.
- **Captures:** the window's own pixels (`ui.screenshot`, 2880 × 1800).
- **The logs:** `logs/a`, `logs/b`, with the bridge's stdout (JSON-RPC, payloads summarized) and stderr. Paths are replaced by `<repo>` and `<scratch>`.

## Run A: Scrim closed, then opened (start order)

| Time | What happened |
|---|---|
| 1.15 s | The app is opened once and gives the entry. It quits through its own Quit (`app.quit`): it removes its endpoint note and leaves its tool list (`mcp-tools.json`) |
| 1.21 s | **The entry is started with Scrim closed.** `initialize` is answered (server "scrim", `tools.listChanged: true`), and `tools/list` gives the 25 tools Scrim last noted. A tool call gets `isError: true`, "Scrim is not running. Open the Scrim app and try again." |
| 6.22 s | The bridge is still running. Its stderr: "Scrim isn't running (no answer from 127.0.0.1:7690); waiting for it" |
| 6.53 s | **Scrim is opened** (port 7690) |
| 7.22 s | **The same bridge process,** never restarted, sends `notifications/tools/list_changed`. Its stderr: "connected to Scrim at 127.0.0.1:7690" |
| 7.25–7.26 s | `inspect_document` and `run_command frame.create` go through |
| 9.49 s | stdin closed: the bridge exits with code 0 |

`a-window-after-scrim-opened.png`:
- the frame "Made through scrim mcp-bridge" in the window;
- the AI tab: "Connected · claude-ai (via scrim mcp-bridge)", "claude-ai (via scrim mcp-bridge) is working…", "Following claude-ai (via scrim mcp-bridge)…";
- the working dots above the frame.

## Run B: the same, with Scrim on a fallback port

The same, except that **before Scrim opens, something else holds 7690** (it accepts connections and never answers). So Scrim takes the next port, 7691.

| Time | What happened |
|---|---|
| 0.35 s | 7690 is held; Scrim is closed |
| 3.37 s | `initialize`, `tools/list` (25) and the tool call ("Scrim is not running…") are answered. The bridge gave the program on 7690 3 s to answer as Scrim; it didn't |
| 8.85 s | **Scrim opens on 7691,** its fallback, and notes it in `mcp-endpoint.json` |
| 9.39 s | **The same bridge** sends `tools/list_changed`; stderr: "connected to Scrim at 127.0.0.1:7691" |
| 9.40–9.41 s | `inspect_document` and the frame go through, on 7691 |
| 11.63 s | stdin closed: the bridge exits with code 0, ending its session at Scrim |

- `b-window-after-scrim-opened-on-7691.png`: as in run A.
- `b-window-after-the-bridge-closed.png`: the "is working" line and the dots are gone, because the session ended. "Connected" stays for 10 minutes after an `initialize`, as before.

**Found in the first attempt of run B, and fixed (`1d19889`):** the bridge's first `initialize` waited on the held port for the tool calls' 300 s timeout. It now gives the app 3 s, and only an answer from a server named "scrim" counts.

Conditions: load 27.3 (run A) and 23.5 (run B), no swap-outs (`logs/*/conditions.json`). No timings are claimed.

## Not verified

- **The real Claude Desktop app** wasn't driven. Its own handling of `tools/list_changed`, and whether it shows "couldn't start" for a bridge that answers, weren't seen. **Please retest:** Re-register Claude Desktop in Scrim (it is offered the upgrade from the npx entry), quit Scrim, restart Claude Desktop, then open Scrim.
- **Authentication on, live:** covered by `crates/mcp/tests/bridge.rs` only. The run had no token.
- **The entry written into a Claude Desktop file** by the window's Re-register: covered by `desktop_flow_tests.rs` on scratch files. The real file wasn't touched.
- **Windows and Linux:** not run.
- **An app that crashed** leaves its note behind. The bridge then tries that port, gets no answer and waits, as with a closed app. That was covered by tests (a port nobody answers), not by a live crash.
