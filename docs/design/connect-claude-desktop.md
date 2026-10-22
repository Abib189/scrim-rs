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

## F1 as built

- **Engine:** `crates/engine/src/cmd/claude_desktop.rs` holds the path per OS, `npx_command`, `entry`, `inspect`, `changed`, `apply`, `stamp`, `report` and the six commands. `claude_desktop/splice.rs` changes one member of `mcpServers` in the text and pretty-prints entries with `command`, `args`, `env` first.
  - **Access:** `mcp.claude-desktop-status` is Read; the other five are Write.
- **Only the desktop app gives the real path.** `McpEndpoint::desktop_config` is set from `Services::claude_desktop_config` when the endpoint starts. The browser, headless runs and every test have none (or a scratch file), so no test can read or write the real configuration.
- **Window:** `crates/ui-egui/src/desktop_flow.rs` gives the Claude Desktop section in the AI tab (under Claude Code) and in Settings ▸ MCP (under Connect Claude Code). Each has:
  - a status row read from the file (every 2 s while it shows, at once after a write);
  - "Registered address is out of date" with Re-register;
  - the panel (file, entry, Write or Re-register, Copy, Close, and the notes: Linux path inferred, npx missing, token written, the file can't be read);
  - the three confirmations, which read the file again and don't ask when there's nothing to do.
- **The outcome** names the backup and says to quit Claude Desktop completely and open it again.
- **Strings:** en and es.
- **Agents can't look or write:**
  - **Reading:** the status reads through `file_access`, so inside an agent's request (confined to its folders) the file is reported as unreadable, not read. An agent learns the path and the entry Scrim would write, not what the file holds.
  - **Writing:** `apply` refuses to run inside such a scope at all. Elsewhere it is allowed direct file access (`#[allow(clippy::disallowed_methods)]`, as `plugin_directory` is), for the backup and the rename.

## F2 results

Captures and the bridge check: `docs/scrim-ports/settings/f2/README.md`.

- **Live in the release build** (a scratch HOME; the real configuration never touched): button, panel, confirmation, written, Configured, Out of date with Re-register and its confirmation, Can't read with Copy only (the file untouched), the old entry offered, and Settings ▸ MCP.
- **The written entry, started as Claude Desktop starts it,** reached Scrim through `mcp-remote@0.14.3` on the live port (7681 here): `initialize`, 24 tools, `inspect_document`.
- **Found and fixed in F2:** a write's message stayed on screen after the file changed (it showed "Scrim was added" under the "can't be read" error). A message now shows only while the file is in the state the write left.
- **Not verified:** a real Claude Desktop (no live test was run against it); Windows and Linux beyond unit tests; no npx, and authentication on, live.

## Fix P: a bridge that waits for Scrim (22 Oct 2026)

**The report:** Claude Desktop started before Scrim. Its entry, `npx -y mcp-remote@0.14.3 http://127.0.0.1:7680/mcp`, found nothing listening and exited, and Desktop said "scrim couldn't start". The same failure would follow a port fallback, because the entry names the port it was written with.

**P0, from Desktop's log** (`~/Library/Logs/Claude/mcp-server-scrim.log`, read for this, not copied): **start order.**
- **22:56:56 UTC on 21 Oct:** Desktop started the server. Three `mcp-remote` processes in turn were refused on 127.0.0.1:7680 (`ECONNREFUSED`), each ended with a fatal error, and Desktop reported "Couldn't start".
- **22:58:30 UTC:** the Scrim app launched (its recovery lock was made then).
- **Afterwards:** the next `mcp-remote` connected to the same port 7680. So Scrim was on its usual port, not a fallback.
- **A port change wasn't the cause this time.** It would fail the same way, which is why the bridge follows the port too.

**What changed:**
- **`scrim mcp-bridge`** (`figcraft_mcp::bridge`; also the app's own binary, `figcraft mcp-bridge`, which opens no window): a stdio MCP server forwarding to the app's HTTP endpoint.
  - **It finds the app before every connection attempt:** `mcp-endpoint.json` in the settings folder, else the port saved in Settings ▸ MCP.
    - The app writes the note (address, port, pid) atomically when its endpoint starts, and removes it when it stops or quits cleanly. Only the process that wrote it removes it.
    - It holds no token.
  - **While the app is closed, it waits:**
    - it answers `initialize` (with `tools.listChanged`), and `tools/list` from `mcp-tools.json` (the app's list, noted on each start and when Tool access changes), else an empty list;
    - each tool call gets the tool error "Scrim is not running. Open the Scrim app and try again.";
    - it tries every 2 s. Once the app is up, it initializes there for the client and sends `notifications/tools/list_changed`.
  - **The app restarting,** on the same port or another, is found by the next request or the retry.
  - **The client's name** reaches the app as `<name> (via scrim mcp-bridge)`, as `mcp-remote` added its own.
  - **The token:** it comes from `SCRIM_MCP_AUTH`, the same variable the entry already used, goes in each request's header, and is never printed.
  - **When stdin closes,** it ends its session at the app (`DELETE /mcp`) and exits.
  - **No Node.js:** it is Scrim's own binary.
- **Connect Claude Desktop writes Scrim's bridge** when the app knows its binary (`Services::bridge_command`: the app's own executable, by its real path):
  - **The entry:** `{"command": "<binary>", "args": ["mcp-bridge"]}`, plus `env.SCRIM_MCP_AUTH` with authentication on.
  - **The status:** "Configured · waits for Scrim to start".
  - **An entry from another copy of the binary** is Out of date, with Re-register.
  - **Without the binary** (the browser, headless): the `mcp-remote` entry, Copy only.
- **An existing `mcp-remote` entry** (any pin) is Out of date, with "Claude Desktop uses the npx bridge", and Re-register upgrades it. As before, that takes one confirmation, makes a backup and writes atomically.
- **The other clients:** every file-based one has a URL entry, so none changes. The generic "stdio" snippet names the bridge when the binary is known.

**Decisions (overrule any):**
1. **The entry runs the app's binary,** not a separate `scrim` CLI. The CLI isn't shipped in the app bundle (F decision 9), and the app always knows its own path. `scrim mcp-bridge` does the same, for people who install the CLI.
2. **The tool list while the app is closed** is the app's last list, else empty, as the brief says. It isn't compiled into the bridge, so that Tool access stays the app's.
3. **The note's address is loopback only:** anything else is ignored, and the saved port is used.
4. **A tool call while the app is closed is not queued until it comes up:** it is answered with the error at once, and the agent can try again.

**Found by the live check (P3) and fixed:** with Scrim closed, something else holding the saved port (it accepted the connection, never answered) held the bridge's first `initialize` for the tool calls' 300 s timeout.
- **`initialize` at the app now has 3 s,** and only an answer naming the server "scrim" counts as Scrim. Tool calls keep 300 s (a big export or render).
- **So the bridge answers at once and keeps waiting,** and follows the app to its fallback port when it comes up.
- **The test:** `a_port_held_by_something_else_doesnt_hold_the_bridge`.

**Tests:**
- `crates/mcp/tests/bridge.rs`: down then up, a new port, the token, stdin closing, the discovery files, a port held by something else;
- `claude_desktop_tests.rs` (2 new): the entry, the upgrade, the report;
- `desktop_flow_tests.rs`: writing the bridge, the upgrade with backup, a moved binary, Copy only without it;
- `mcp_endpoint_tests.rs`: the app's note, which follows a restart and goes away when the endpoint is dropped;
- `agent_clients_tests.rs`: the stdio snippet.



## Fix S5: the bridge across Scrim restarts (22 Oct 2026)

**Checked live** (`docs/scrim-ports/connect-agents/s5-bridge/`).
- **Setup:** the real bridge (debug builds), run as both `figcraft mcp-bridge` and the `scrim` CLI's `scrim mcp-bridge`, each in its own run with the same results. Both used a scratch HOME and settings folder, with a stub client on stdin and stdout, connected throughout. The table is the `figcraft` run; the `scrim` run's figures are in the folder's README.
- **The app quit and started again three times:**
  - on the same port (7690);
  - on another port (7691);
  - on the same port with no call while it was down.
- **Calls ran one after another across each quit,** so that one was at the app as it went.

| Check | Same port | Another port | Same port, quiet |
|---|---|---|---|
| The call in flight at the quit | "Scrim is not running…", 2 ms | the same, 2 ms | no call made |
| Every call while closed | "Scrim is not running…", 1 ms or less | the same | — |
| `notifications/tools/list_changed` | yes, as the app came up | yes, 0.9 s after | **before the fix: never**; after: 1.5 s after |
| The next call, without restarting the client | answered | answered | answered |

- No call took more than 38 ms.
- The app quit in 0.01–0.17 s each time.

**Found and fixed: a quiet restart on the same port went unnoticed.**
- **The cause:** the app takes a session id it doesn't know as the newest client's. So the bridge's old session went on answering, and the bridge's 404 branch never fired.
- **The effect:** the client was never told the tool list might have changed, and the new app never got the client's `initialize`. It didn't know the client's name, so it couldn't say "{client} is working".
- **The fix:** the bridge now remembers which app instance the note names (`mcp-endpoint.json` › `pid`, `discovery::instance`) when it connects.
  - **When it checks:** every retry (2 s) while the link is up, and before every request.
  - **On a new instance:** it initializes again for the client and sends `list_changed`.
  - **A note that appears after the bridge connected** is taken as the same instance's.
- **Why not a 404:** answering an unknown session with 404, as the spec allows, would change the endpoint for every client, Claude Desktop's mcp-remote among them. The bridge was the narrower place.
- **Tests:** `crates/mcp/tests/bridge.rs` ›
  - `a_restart_on_the_same_port_with_no_call_meanwhile_is_found_and_the_client_told` fails with the check disabled;
  - `an_app_that_notes_no_instance_is_never_taken_for_a_restart`.

**Not seen, not changed:** a request already queued for the window when the window closes. If its reply channel closes before the process exits, the app's endpoint (`mcp_endpoint.rs`) answers "timed out waiting for the Scrim window…" rather than "Scrim is not running". In every live quit the process went first, and the bridge answered "Scrim is not running".

**Not verified:**
- Claude Desktop itself across a restart. The stub stands in for it, and Desktop's handling of `list_changed` is its own.
- A crash: the note stays behind with a dead pid, and the next request finds nothing listening.


## Fix T4: where Claude Desktop's block lives now (22 Oct 2026)

Register, Re-register and the block's status left the AI tab for **Settings ▸ Agents**, with the same commands (`mcp.connect-claude-desktop`, `mcp.write-claude-desktop`). They are also still on Settings ▸ MCP. The AI tab is a chat (`docs/design/ai-panel.md`).
