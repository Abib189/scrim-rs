# Stage C captures (20 Oct 2026)

These are real window captures (`ui.screenshot`, the window's own pixels), not offscreen renders. They were taken from the release build of `8a68c7a` with text visible.

- **01–11:** a scratch HOME (so the owner's settings were untouched), 1920 × 1050 points at 1× (the menu bar takes the rest of the 1080), light theme, a copy of `examples/ridgeline-app.figcraft`.
- **12–14:** the owner's real HOME, for the real Connect (2× display, so 3840 or 3420 px wide).
- **The fonts pair:** a scratch HOME, so the fonts installed in `~/Library/Fonts` aren't seen, which is how a machine without them behaves.
- **Redactions:** two local paths were painted over in the background colour (01's open-file path, 05's home folder).

| File | What it shows |
|---|---|
| `01-home-sidebar.png` | Home: Settings pinned at the bottom of the sidebar, below a hairline; the rows above where they were |
| `02-settings-general.png` | Settings ▸ General (Appearance, Animations, Shared rooms, …) |
| `03-settings-toolbar.png` | Settings ▸ Toolbar: groups, Show, Up / Down / Join above / Own button, Reset to default |
| `04-settings-ai.png` | Settings ▸ AI & agents: Claude Code's status and Connect, no keys or models |
| `05-settings-mcp.png` | Settings ▸ MCP: Running, Restart, address and Copy, version, Require authentication, files folder, Connect |
| `06-settings-tools.png` | Settings ▸ Tool access: 24 of 24 enabled, search, the read-only group |
| `07-settings-privacy.png` | Settings ▸ Privacy: the statement |
| `08-settings-diagnostics.png` | Settings ▸ Diagnostics: Keep for, Copy / Save… / Clear, one event |
| `09-ai-tab-empty.png` | The AI tab before connecting: Not connected, Connect Claude Code |
| `10-ai-tab-connect-command.png` | The connect flow: the exact command, Run, Copy, Close |
| `11-connect-confirmation.png` | "Run this command?", the one confirmation |
| `12-real-connect-confirmation.png` | The same, in the owner's app before the real run |
| `13-real-connect-added.png` | After Run: "Scrim was added to Claude Code for every project." (green then; now in the text colour, see C4) |
| `14-real-ai-tab-connected.png` | "Connected · claude-code" after Claude Code's health check |
| `fonts-before-missing-fonts.png` | Before bundling: "Brand colour system mockups" opens with Missing fonts (Geist, Inter, Source Serif 4) |
| `fonts-after-no-dialog.png` | After: the same file opens with no dialog |

`c4-live-proofs.md` holds the security, tool-access and feed proofs, with what `claude mcp list` said.
