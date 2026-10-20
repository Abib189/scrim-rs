# Connect Claude Desktop (Stage F)

A one-click "Connect Claude Desktop" beside Connect Claude Code (C2): it writes Scrim into Claude Desktop's configuration, safely, after one confirmation. F0 below is what the documentation says and how C2 is built; F1 and F2 add what was built and verified.

## F0. How Claude Desktop connects to a local MCP server

Checked on 20 October 2026. modelcontextprotocol.io failed here with a certificate error (this network serves another site's certificate for it), so the MCP guides were read from their source in `github.com/modelcontextprotocol/modelcontextprotocol`, which is the site's content.

| Source | Version read |
|---|---|
| MCP docs, *Connect to local MCP servers* (`docs/docs/2026-07-28/develop/connect-local-servers.mdx`) | commit `1f520964b2`, 11 Oct 2026 |
| MCP docs, *Connect to remote MCP servers* (`docs/docs/2026-07-28/develop/connect-remote-servers.mdx`) | commit `046fa30efd`, 28 Sep 2026 |
| Anthropic Help Center, *Get started with custom connectors using remote MCP* (`support.claude.com/en/articles/11175166`) | "Updated over 2 weeks ago", read 20 Oct 2026 |
| `mcp-remote` README (`github.com/punkpeye/mcp-remote`; it began as `geelen/mcp-remote`) and its npm metadata | npm `latest` 0.15.2, published 20 Oct 2026 |

### What the documentation says

1. **The configuration file** (local guide, "Access Developer Settings"): Settings ▸ Developer ▸ Edit Config "creates a new configuration file if one doesn't exist":
   - **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
   - **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
   - **Linux:** the guide says "Claude Desktop is available for macOS, Windows, and Linux", but gives **no Linux path**.
2. **The format:** `{"mcpServers": {"<name>": {"command": …, "args": […], "env": {…}}}}`. These are **stdio servers only**: Claude Desktop starts the command. The guide shows no `url` or HTTP form for this file.
3. **Remote (HTTP) servers are Custom Connectors**, added in Settings ▸ Connectors. The Help Center: "Claude connects to your remote MCP server from Anthropic's cloud infrastructure, rather than from your local device". For Claude Desktop, "the connection to your MCP server originates from Anthropic's servers, not from your machine's network interface". So the server must be reachable from the public internet.
   - **Scrim's endpoint is `http://127.0.0.1:<port>/mcp`.** A connector can't reach it.
   - The same article: servers in `claude_desktop_config.json` are "a separate mechanism and do use your local network".
4. **So Desktop needs a stdio bridge to Scrim's HTTP endpoint.** `mcp-remote` is one: a stdio MCP server that forwards to a streamable-HTTP or SSE URL.
   - Its README shows exactly this use in Claude Desktop's format (`"command": "npx", "args": ["mcp-remote", "<url>"]`).
   - It allows plain HTTP on loopback (`--allow-http` is only for other private hosts).
   - It is **not an Anthropic project:** MIT, maintained by punkpeye (glama.ai), 124 versions on npm.
5. **After editing:** "completely quit Claude Desktop and restart it". Its MCP logs are in `~/Library/Logs/Claude` (macOS) and `%APPDATA%\Claude\logs` (Windows).

**Where the documentation and the brief differ:**
- **The Linux path.** The brief allows "Linux if supported"; the docs say Desktop runs on Linux without naming the file. Scrim uses `$XDG_CONFIG_HOME/Claude/claude_desktop_config.json` (else `~/.config/Claude/…`). That is the Electron default for an app's data folder, the same rule that gives the documented macOS and Windows paths, so it is an **inference, not a documented path**, and the flow says so on Linux.
- **The Windows Store build.** Third-party guides say it uses `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude\`. Anthropic's docs don't, so Scrim doesn't guess it. The flow shows the exact path it will write.

The rest agrees with the brief: the file paths, and the `npx -y mcp-remote <url>` bridge.

## How Connect Claude Code (C2, C1.2) is built

- **The engine** (`crates/engine/src/cmd/mcp.rs`):
  - **`mcp.connect-claude-code`** asks the window to show the flow, with the exact command (`claude mcp add --transport http --scope user scrim <url>`).
  - **`mcp.run-connect`, `mcp.reregister` and `mcp.remove-old-entry`** only *ask*: they request `dialog.*` UI. An agent's commands never open a dialog, so **an agent can't make Scrim act on its own**.
  - **`mcp::status`** gives the live address: the port the endpoint really listens on (7680 or the fallback), the token when authentication is on.
- **The window:**
  - **`connect_flow.rs`** holds the panel (command, Run and Copy, progress, outcome) and the three confirmations. After the confirmation it runs `claude` with an argument list on a background thread (`Services::run_program`).
  - **`mcp_registration.rs`** is the read-only check, `claude mcp get scrim`, and the row "Registered address is out of date" with Re-register.
  - **`ai_tab.rs` and `settings_mcp.rs`** show the button, the row and the panel.

### The plan: the same flow, a second target

Claude Desktop can't be asked anything (no CLI), so this target reads and writes a file where Claude Code runs a program. Everything else is shared in shape:
- the button that opens a panel with the exact change;
- Write and Copy;
- **one confirmation** (a `dialog.*` request, so agents can't write);
- **Re-register** when the address is out of date, with one confirmation;
- **Remove the old entry** (`figcraft`), with its own confirmation;
- a status row.

**What is built:**
- **`crates/engine/src/cmd/claude_desktop.rs`:** the file's path per OS, the entry, the status from the file, and the safe write. These are pure functions plus `std::fs`, testable on a temporary folder. Also the commands:
  - `mcp.connect-claude-desktop` (the MCP route; returns the path, the entry and the status);
  - `mcp.write-claude-desktop`, `mcp.reregister-claude-desktop` and `mcp.remove-old-desktop-entry` (each asks);
  - `mcp.copy-claude-desktop-entry`.
- **`crates/ui-egui/src/desktop_flow.rs`:** the panel and its confirmations, beside `connect_flow.rs`. It reuses its widgets and patterns, and the same status source (`mcp::status`) for the live address.
- **The AI tab and Settings ▸ MCP** get "Connect Claude Desktop" beside "Connect Claude Code", and a status row: Configured, Not configured, Out of date, or Can't read the file. It is read from the file; never "Connected", since Desktop can't be asked.

## Decisions for F1 (overrule any)

1. **`mcp-remote` is pinned:** `npx -y mcp-remote@0.14.3 <url>`.
   - Unpinned, `npx -y` fetches whatever npm has as newest every time Claude Desktop starts, a third-party package running with the person's permissions.
   - 0.14.3 was published on 21 Sep 2026. 0.15.2, today's, is too new to trust blindly.
   - Moving the pin is a one-line change, and **Re-register** updates entries written with an older pin.
2. **On macOS and Linux, the entry names npx by its full path,** with `PATH` set to npx's folder plus the system folders (`env.PATH`).
   - A desktop app gets a short `PATH`: npx from nvm or Homebrew isn't on it, and npx needs `node` from its own folder.
   - On Windows it is `"npx"`, as in Anthropic's example.
3. **Without npx, there's no Write:** only Copy, with "Node.js's npx wasn't found". The bridge needs it.
4. **With MCP authentication on,** the token goes into the entry as `--header Authorization:${SCRIM_MCP_AUTH}` with `env.SCRIM_MCP_AUTH = "Bearer <token>"`. This uses the README's workaround for Claude Desktop on Windows, which doesn't escape spaces in `args`.
   - The flow says the token is written into Claude Desktop's file.
   - Re-register writes it again after the token changes.
5. **Formatting is kept where practical:**
   - only `mcpServers.scrim` (or the old entry) is spliced into the existing text;
   - the result is parsed and checked to equal the intended JSON;
   - if that check ever fails, the file is written pretty-printed instead (same meaning, keys sorted).
   - Empty or missing files get pretty-printed JSON.
6. **"Out of date"** is a `scrim` entry whose URL isn't this window's live address. That includes the `scrim mcp` stdio form `docs/mcp.md` used to suggest, and an older `mcp-remote` pin. Re-register replaces it after one confirmation; it is never overwritten silently.
7. **Old entries** are the keys `figcraft` and `figcraft-app` (the names before the rename, `docs/mcp.md`).
8. **Only the person can write:** the write, re-register and removal commands only ask. Agents get the path, the entry and the status, and Copy.
9. **A first-party bridge would remove the Node.js dependency.** That would be a `scrim mcp` mode forwarding stdio to the HTTP endpoint, with the CLI installed beside the app. It isn't built here: the brief names `mcp-remote`, and the CLI isn't shipped inside the app bundle.
