# Stage F verification (F2, 20 Oct 2026): Connect Claude Desktop

Real window captures from the release build (`ui.screenshot`, the window's own pixels; this display is 1×), the Ridgeline example open.

**Your configuration was never touched.** The app ran with a scratch `HOME`, so "Claude Desktop's configuration" was a file inside a scratch folder (`<scratch>/Library/Application Support/Claude/claude_desktop_config.json`). Nothing was read or written in `~/Library/Application Support/Claude/`, and Claude Desktop itself wasn't involved.

**Local paths are painted over:** the scratch folder, and npx's folder under the home folder, which shows a user name. What is left is the rest of the entry as written.

The endpoint listened on **7681**: 7680 was taken on this machine, so the fallback picked the next port. The entry uses that live port.

| Capture | What it shows |
|---|---|
| `f-01-ai-tab-not-configured.png` | The AI tab: Claude Code, then **Claude Desktop: Not configured** and **Connect Claude Desktop** |
| `f-02-panel.png` | The panel: the exact file, and the exact entry (`npx -y mcp-remote@0.14.3 http://127.0.0.1:7681/mcp`, npx by full path, `PATH`) |
| `f-03-panel-buttons.png` | Write, Copy, Close |
| `f-04-confirmation.png` | The one confirmation, naming the file. Before it is answered the file doesn't exist (nor its folder) |
| `f-05-written.png` | After Write: "Scrim was added to Claude Desktop. Quit Claude Desktop completely and open it again to use it." |
| `f-06-status-configured.png` | **Configured**, read back from the file |
| `f-07-status-out-of-date.png` | The file pointed at 7600: **Out of date**, "Registered address is out of date", "Claude Desktop points to http://127.0.0.1:7600/mcp; this window is at http://127.0.0.1:7681/mcp.", **Re-register** |
| `f-08-reregister-confirmation.png` | Re-register's confirmation. After it, the file pointed at 7681, a backup `claude_desktop_config.json.scrim-backup-20261020-190530` was beside it, and the status was Configured |
| `f-09-status-unreadable.png` | The file made invalid JSON: **Can't read its configuration** |
| `f-10-unreadable-copy-only.png` | The panel: the parse error, Copy and Close, no Write. The file's checksum was unchanged and no backup was added |
| `f-12-old-entry-offered.png` | A file with an old `figcraft` entry (and another server): "Claude Desktop still has the old figcraft entry from before the rename." and **Remove the old entry** |
| `f-13-settings-mcp.png` | Settings ▸ MCP ▸ Connect: **Connect Claude Code**, then the Claude Desktop section and **Connect Claude Desktop** |

## The entry works: a bridge check

Claude Desktop starts the entry as a program and speaks MCP to it on stdio. The check did exactly that with the file Scrim wrote (`f-05`): its `command`, `args` and `env`, run by a script standing in for Claude Desktop, with its HOME and npm cache in the scratch folder.
- `npx` fetched `mcp-remote@0.14.3` and started it.
- `initialize` was answered by Scrim (its instructions).
- `tools/list` returned **24 tools**, including `list_commands`, `run_command`, `batch`, `create_node`, `set_properties` and `inspect_document`.
- `inspect_document` returned the open file, "Ridgeline app".
- Scrim recorded the client: the AI tab showed "Connected · claude-desktop-bridge-check (via mcp-remote 0.14.3)".

## Not verified

- **Claude Desktop itself.** No live test against a real Claude Desktop was run: it was neither started nor restarted, nor pointed at the scratch file. The bridge check runs the same command Claude Desktop would start; that Desktop then lists Scrim's tools in its own window was not observed.
- **Windows and Linux:** only their path resolution and entry, by unit tests. The Windows Store install path isn't handled, because Anthropic doesn't document it.
- **No npx, live:** this machine has Node.js. "Copy only" without npx is covered by `without_npx_it_says_so_and_offers_copy_only`.
- **MCP authentication on, live:** the entry's token header is covered by unit tests only.
- **Claude Code's status row shows any client of the endpoint.** In `f-07` and `f-08` it says "Connected · claude-desktop-bridge-check…": the bridge, not Claude Code. That is C2's rule ("a client initialised in the last 10 minutes"), unchanged here. It will also show Claude Desktop once Desktop connects through the bridge.
