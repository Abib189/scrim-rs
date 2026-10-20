# Stage C.1 verification (C1.6, 20 Oct 2026)

Real window captures from the release build (`ui.screenshot`, the window's own pixels at 2×: 1920 points wide), except `keyboard-02-save-panel.png`. That one is a system screen capture (the save panel is the system's window), cropped to the Scrim window. A local path was painted over in `dark-04-settings-mcp.png`.

## Dark theme, every Settings section

`dark-01-settings-general.png` … `dark-07-settings-diagnostics.png` (a scratch HOME; the Ridgeline example open).

## Replace and the old figcraft entry, against the installed `claude` (2.1.296)

The app ran with `CLAUDE_CONFIG_DIR` set to a temporary folder holding an old `scrim` entry (4848) and an old `figcraft` entry, so the owner's own config was never touched. It still points where Stage C left it (`scrim` at 7600, user scope).

| Step | Capture |
|---|---|
| The AI tab: "Registered address is out of date" (Claude Code points to 4848, the window is at 7681) | `replace-01-out-of-date.png` |
| Connect Claude Code ▸ Run ▸ the one confirmation | `replace-02-confirmation.png` |
| The existing entry is offered for Replace, not overwritten | `replace-03-offered.png` |
| Replaced: "Scrim was added…", "Connected · claude-code"; the old figcraft entry is offered for removal | `replace-04-replaced.png` |
| Its own confirmation | `replace-05-remove-old-confirmation.png` |
| "The old figcraft entry was removed from Claude Code." | `replace-06-old-removed.png` |

`claude mcp list` with that config, before:

```
Checking MCP server health…
scrim: http://127.0.0.1:4848/mcp (HTTP) - ✘ Failed to connect — ECONNREFUSED: ECONNREFUSED: Unable to connect. Is the computer able to access the url?
figcraft: figcraft-cli mcp - ✔ Connected
```

and after:

```
Checking MCP server health…
scrim: http://127.0.0.1:7681/mcp (HTTP) - ✔ Connected
```

## Tool access, live, through `scrim mcp` (headless) and `scrim mcp --attach`

The release `figcraft-cli`, the same requests on both routes. Headless used a scratch HOME whose saved settings turn `create_node` off; `--attach` used a running app where `mcp.set-tool` had turned it off.

Headless:

```
tools/list: 23 tools; create_node listed: False
create_node: refused — the tool `create_node` is turned off in Scrim (Settings ▸ Tool access)
run_command node.create: refused — `node.create` does what the tool `create_node` does, and that tool is turned off in Scrim (Settings ▸ Tool access)
batch [node.create]: refused — `node.create` does what the tool `create_node` does, and that tool is turned off in Scrim (Settings ▸ Tool access)
run_command command.batch [node.create]: refused — `node.create` does what the tool `create_node` does, and that tool is turned off in Scrim (Settings ▸ Tool access)
run_command page.list (another tool's): ran (isError=False)
```

`--attach`:

```
tools/list: 23 tools; create_node listed: False
create_node: refused — the tool `create_node` is turned off in Scrim (Settings ▸ Tool access)
run_command node.create: refused — `node.create` does what the tool `create_node` does, and that tool is turned off in Scrim (Settings ▸ Tool access)
batch [node.create]: refused — `node.create` does what the tool `create_node` does, and that tool is turned off in Scrim (Settings ▸ Tool access)
run_command command.batch [node.create]: refused — `node.create` does what the tool `create_node` does, and that tool is turned off in Scrim (Settings ▸ Tool access)
run_command page.list (another tool's): ran (isError=False)
changes in this file: [('person', 'node.set')]
```

(The one change in that file is the person's own `node.set`, from a Space pressed during the keyboard check before Settings was modal; see below.)

## The keyboard alone: Settings and its Save… dialog

1. **Settings opens with Cmd+,.** On macOS that shortcut belongs to the native menu bar, which the control channel reaches as `ui.menu.invoke {id: "app.preferences"}`. `ui.input` key events go to egui, after the menu.
2. **Tab, then Space.** Diagnostics after 8 Tabs, Space picks it (`keyboard-01-settings-diagnostics.png`). Save… after 6 more Tabs, Space opens the system's save panel (`keyboard-02-save-panel.png`).
3. **This found a bug,** fixed in `8cb4790`. Before, Tab walked out of Settings into the tab bar, panels and toolbar behind it, and Space could change the document there. Settings is now modal.

## WASM size, before and after the bundled fonts

| Build | `figcraft-web.wasm` (raw `cargo build --release --target wasm32-unknown-unknown`) |
|---|---|
| `06a2425` (before the fonts) | 48,780,116 bytes |
| `02356cf` (the fonts commit) | 48,676,495 bytes |
| Difference | −103,621 bytes (−0.10 MB) |

The fonts are compiled out of the web build (`crates/text/src/bundled.rs` is empty on wasm32), so there is no growth. Nothing needs loading lazily (the threshold was 1 MB).
