# Fix W6: following on by default, and agents can't change a setting, live (23 Oct 2026)

`../w6live.py BINARY SCRATCH OUT` runs the debug build of this tree twice, on one scratch HOME and one scratch settings folder, with MCP on 7690 and the control channel on 7979. The folder starts with the real settings file a Fix T build wrote, from before Fix V (`apps/figcraft/tests/fixtures/settings/state-v8-fix-t.json`): `settings_version` 8, following saved **off**, and no `follow_chosen` marker, as an agent left it before Fix V2. A stub MCP client is named as Codex names itself (`codex-mcp-client 0.163.0-alpha.11`). The figures are in `report.txt`; the samples are in `w6.json`.

**The screen was unlocked**, so the window was visible: the figures are the real window's state, read through its control channel. No screenshot was taken.

| Check | Result |
|---|---|
| First launch | following **ON**: the engine says "following", the header reads ON. Diagnostics' first line: "follow setting: OFF → ON (made by a settings migration: saved off was never chosen)". Saved as version 9, `follow_chosen` still false |
| The agent asks to change the reveal, its speed, the dots, the theme, the accent, the motion and following (twice) | **8 of 8 refused**, each noted: "setting pref.revealAgentWork: unchanged (refused: an agent asked, over MCP, session 40aa5876…)", and so on; following's "follow setting: stays ON (refused: …)". Every setting saved as it was |
| `finish_working` | works |
| `mcp.set-tool` export off | works (`disabled_tools: ["export"]`) |
| `mcp.set-tool` export on | **refused** and noted |
| The build: an artboard, then a batch of 40 rectangles in it, 3,000 units from the view | the camera flew (5 captures mid-flight, the zoom 1.0 → 0.860) and centred the 40 rectangles to **0.00 px** |
| The reveal | **41 entered** (the artboard and the 40), 0 drawn final at once; all 41 held for the flight, then entered |
| The dots | on in 30 of 31 captures while the agent worked. The one without was taken 0.03 s after the build started, before the window had drawn the artboard. **Every capture from the first frame drawn (0.17 s) until `finish_working` has them** |
| Second launch | following still ON, still one repair line; the only new line is "started on 127.0.0.1:7690"; every setting as the first launch left it |

**Not verified:**
- the look of the header or the dots (no screenshot);
- the person's own click on a Settings control in the real app (the UI test covers it);
- the CLI and plugin routes live (the engine tests cover them);
- the person's own settings. They weren't read or changed: the repair runs inside the app when the person next launches it.
