# Fix S4 live runs: who moved the camera

`s4.py BINARY SCRATCH OUT [editor|home]` starts the app on a scratch HOME and a scratch settings folder (`SCRIM_SETTINGS_DIR`), with MCP on 7690 and the control channel on 7979.

- **The agent** is an MCP client on the app's real HTTP endpoint, named `local-agent-mode-scrim (via mcp-remote 0.14.3)`, as Claude Desktop's bridge names itself. It opens a saved file with `open_file`, then sends six batches at x 3,000 to 6,500, 4 s apart.
- **The control channel** only sets the start and samples `ui.inspect` every 150 ms (`follow`, `pausedBy`, the newest camera change). Settings ▸ Diagnostics is read from the scratch settings file after the app quits, never the clipboard.

| Run | Samples | Pauses | Diagnostics (follow) |
|---|---|---|---|
| `s4-editor.json`: the file in front | 197 | none | none |
| `s4-home.json`: Home in front (the agent's file opens behind it, so nothing is followed) | 201 | none | none |

**In the editor run, the window stopped drawing at about 10 s.** It is a window on the owner's screen, and other windows covered it. The later batches were answered without frames (`control::drain_hidden`), and following didn't run until frames came back. The harness covers the same case (`follow_pause_cause_tests.rs`, `HiddenAfterOpen` and `HiddenThroughout`).

**Debug build, 22 Oct 2026, machine under load.** No timings are claimed.
