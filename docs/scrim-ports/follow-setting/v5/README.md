# Fix V5: the Follow agents setting live, after the fix (22 Oct 2026)

`../v5live.py BINARY SCRATCH OUT`:
- **The app:** the debug build of this tree, on a scratch HOME and a scratch settings folder, with MCP on 7690 and the control channel on 7979. It starts from the settings file the build before Fix T wrote (`apps/figcraft/tests/fixtures/settings/state-v7-before-fix-t.json`: version 7, Follow on, an unmarked 30 s grace).
- **The agent:** a stub MCP client on the app's HTTP endpoint, named as Codex names itself. It first asks `camera.follow-agents {}`, the call that turned following off before Fix V2. Then it builds 40 layers 3,000 units from the view: an artboard, then 39 rectangles in it in one `batch`. Then it calls `finish_working`.
- **The figures** come from `ui.inspect` and `view.get`, read every 100 ms, and from Settings ▸ Diagnostics in the settings file the app saved. They are in `report.txt`; the samples are in `v5.json`.

**The screen was locked** (`CGSSessionScreenIsLocked`). No screenshot was taken: every figure is the app's own state, read through its control channel. macOS still counted the window as visible this time, unlike in T6, so the reveal ran.

| Check | Result |
|---|---|
| Follow at start, from the pre-Fix-T file | **ON**: the engine says "following"; the header's button reads ON, not dimmed, with the tooltip "Follow agents as they work". The file was migrated to version 8 |
| The agent's `camera.follow-agents {}` | **refused**, "only the person turns following on or off: …"; following still on |
| The camera | flew: 6 captures mid-flight, the zoom from 1.0 to 0.853 |
| Centred | **0.00 px** between the canvas's centre and the 39 rectangles of the batch. The artboard landed one pass earlier, from its own request, so the camera centred on the latest pass's changes (Fix T2's rule); the whole artboard's centre is 47.6 px away |
| The reveal | **40 entered, 0 drawn final at once** (`ui.inspect`'s counters since the window opened). All 40 entrances were held during the flight (0.11–0.66 s), then entered (T3) |
| The dots | on in 36 of 36 captures between the first layer and `finish_working` |
| Diagnostics | "codex-mcp-client 0.163.0-alpha.11 connected", "**follow setting: stays ON (refused: an agent asked, over MCP, session b0f6dadc…)**", "begin: … first request", "end: … finish_working". No "following is OFF", no change of the setting |
| Saved | `follow_agents: true`, `settings_version` 8 |

**Not verified on a real window** (the screen was locked):
- the header's look, ON and OFF;
- the entrances and the dots as drawn;
- a person's click on the crosshair or the toggle, and its line in Settings ▸ Diagnostics (the UI tests cover these);
- the startup line "following is OFF" in the real app (an app test covers it).
