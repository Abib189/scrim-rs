# Stage J verification (J3, 21 Oct 2026): connect any agent

**The setup:**
- **The app:** the release build, started with a scratch `HOME` (`<scratch>/home-j3`). Every client configuration it read or wrote is a file inside that folder. Nothing was read or written in your real `~/.codex`, `~/.cursor` or `~/.config`, or in any other app's real configuration.
- **Captures:** `ui.screenshot`, the window's own pixels.
  - Captures 01–14 are 2880 × 1800 (2×); the display changed scale between runs, so 15–25 are 1440 × 900 (1×).
  - **04, 07, 12 and 14 were retaken in Stage K3** (1440 × 900, 1×). They come from the build with the current wording and K1's two labels, on a fresh scratch home (`home-k3`), through the same flows, clicked in the window. The backup in 12 is therefore named `…-150731`.
- **The endpoint** listened on 127.0.0.1:**7681**: another Scrim held 7680, so the fallback picked the next port. Every command and entry uses that live port.
- **Driving the window:**
  - Every Connect, Run, Write and Re-register was **clicked in the window** (`ui.input`), like the person's own clicks.
  - The control channel only opened panels, scrolled and captured. It counts as an agent: when it tried to turn authentication off, Scrim refused, as it should.
- **Local paths are painted over** in 09–12 and 14–16, where the scratch folder's path shows your user name. What's left is the client's own part of the path (`home-j3/.cursor/mcp.json`, …).

## Codex, live (the real `codex` 0.162.1 CLI)

`codex-before.txt` and `codex-after.txt` hold Codex's own answers, with the scratch path replaced by `<scratch>`.

| Step | Result |
|---|---|
| **Before** | `HOME=<scratch> codex mcp list --json` → `[]` |
| `01-ai-tab-list.png` | The AI tab: Claude Code and Claude Desktop keep their sections. Under them, **Connect other agents** lists what was found on this Mac (Codex, Cursor, VS Code, Goose: their programs are installed); the rest are under **More (10)**. Each row: Not configured / Not checked · Docs only · 2026-10-21 |
| `02-codex-panel.png` | Connect (clicked): the exact command, `codex mcp add scrim --url http://127.0.0.1:7681/mcp`, then Run, Copy, Close, Documentation |
| `03-confirmation.png` | Run (clicked): **one confirmation**, with the command, "without a shell", and "Scrim doesn't restart Codex". Before it was answered, `<scratch>/.codex/` held only Codex's own `tmp` folder |
| `04-added.png` | Run (in the dialog): Codex's own command ran (an argument list). **Configured**, read back from `~/.codex/config.toml`, and "Scrim was added to Codex. Restart Codex to use it." |
| **After** | `codex mcp list --json` → one server, `scrim`, `streamable_http`, `url: http://127.0.0.1:7681/mcp`, enabled; `codex mcp get scrim` agrees |
| `05-out-of-date.png` | The entry was pointed at 7600 by hand: **Out of date**, "Codex points to http://127.0.0.1:7600/mcp; this window is at http://127.0.0.1:7681/mcp.", and Re-register with both command lines (remove, then add) |
| `06-reregister-confirmation.png`, `07-reregistered.png` | Re-register's confirmation names what it replaces. After it: Configured, and `codex mcp list` → `[["scrim", "http://127.0.0.1:7681/mcp"]]` |
| `23-codex-no-token.png` | MCP authentication on: Codex's entry has the right address but no token, so it's Out of date, said as "Codex has this address but not the token, and MCP authentication is on." The command adds `--bearer-token-env-var SCRIM_MCP_TOKEN` |
| `24-…confirmation.png`, `25-codex-token-configured.png` | Re-registered: `codex mcp get scrim` → `bearer_token_env_var: SCRIM_MCP_TOKEN`. The file names the variable; the token itself is in no file. The app reads it back as Configured |

**Codex is now labelled "verified live"** (`clients.json`): its add, remove and token option ran for real, and Codex's own list and get confirmed each one.

## File clients, live on scratch files (the clients themselves not started)

| Capture | What it shows |
|---|---|
| `09-cursor-panel.png`, `10-cursor-panel-buttons.png` | Cursor: the exact file (`~/.cursor/mcp.json`) and entry; Write, Copy, Close |
| `11-cursor-confirmation.png` | One confirmation, naming the file. Before it was answered, the file was byte-for-byte what it had been |
| `12-cursor-written.png` | After Write: Configured, and "…The previous file was kept as mcp.json.scrim-backup-20261021-150731" (K3's retake) The file kept its `// my servers` comment and its `github` server as written; Scrim's entry was added beside them; the backup is the original byte for byte |
| `13-every-status.png` | Every status at once: Codex and Cursor **Configured**, VS Code **Not configured**, Zed **Can't read its configuration** (an invalid file; Zed was found by its folder), Goose **Not checked** (a YAML client, Copy only) |
| `14-zed-unreadable-copy-only.png` | Zed's panel for the invalid file: the parse error in red, Copy and Close, no Write. The file's checksum was unchanged and no backup was made |
| `15-zed-token-warning.png` | Authentication on, Zed (no documented way to read a token from elsewhere): the entry shows **`Bearer <token>`** (the window never paints the token itself) and the amber warning |
| `16-plain-token-confirmation.png` | Its confirmation says the token will be stored as plain text, and is answered with **Write the token**. Cancelled: the file was unchanged |
| `17-auth-on-entries-out-of-date.png` | With authentication on, the entries written before it (Codex, Cursor) are Out of date: they don't send the token |
| `18-other-agent-card.png` | **Other agent:** the address, JSON and TOML (with `SCRIM_MCP_TOKEN`, not the token), the stdio command (`scrim mcp --attach 7979`), the token note, and why an agent in the cloud can't reach this computer |

## A non-Claude name in the AI tab

An MCP client connected to the real endpoint announcing Codex's `clientInfo` name, `codex-mcp-client` (a script, `client.py`, not Codex itself), and made one edit (`run_command frame.create`). The endpoint issued it an `Mcp-Session-Id`.

| Capture | What it shows |
|---|---|
| `19-ai-tab-codex-named.png` | "Connected · **Codex**" |
| `20-activity-codex.png` | "Following **Codex** · Poster", and the Activity row "**Codex**: Create frame" with Undo |
| `21-settings-mcp.png` | Settings ▸ MCP ▸ Connect: the same section (one implementation) |
| `22-resume-inside-the-panel.png` | Paused, the AI tab's **Resume** button sits inside the panel (see below) |

## Found and fixed while verifying

- **The Connect button was pushed out of the panel** by long names (seen in the first run, recaptured after the fix): the name and the button are now one line, and the status and verification a second.
- **Close was cut off** by Documentation in the panel's button row: Documentation has its own row.
- **The panel painted the MCP token** in a literal-token entry (Zed): the window now masks it as `<token>`, as Claude Code's command does. Write and Copy still use the real token. The capture that showed it was deleted, not kept.
- **"points to http://127.0.0.1:7681/mcp; this window is at http://127.0.0.1:7681/mcp"** for an entry missing only the token: it now says "has this address but not the token".
- **"Scrim will run / will add…"** showed even when Scrim wouldn't (Configured, Can't read): the intro now says it only when it will.
  - Captures `04`, `07`, `12` and `14` were first taken before this change. They were retaken in Stage K3 with the current wording: "Codex's own command, which adds Scrim for your user:" and "Scrim's entry in Cursor's / Zed's configuration file:".
- **Stage I's Resume button** was pushed out of the AI tab whenever the tab scrolled (its Follow switch takes the width): Resume now sits at the row's end. Covered by `follow_agent_tests.rs` › `resume_stays_inside_the_ai_tab_when_it_scrolls`.

## Not verified

- **A Codex session using Scrim's tools.** Codex's registration was verified with Codex's own CLI. Starting a Codex session needs your OpenAI sign-in, which the scratch home doesn't have and this run didn't copy. The tool calls over the endpoint were made by `client.py`, not Codex.
- **Cursor, Zed, VS Code, Goose and every client under More:** **docs only.** Cursor's and Zed's files were written or refused live on scratch files, but neither app was started, so whether they then list Scrim's tools wasn't observed.
- **VS Code's `code --add-mcp`** wasn't run. It talks to a running VS Code, and this session runs inside your VS Code. Its arguments are covered by unit tests.
- **Not installed here:** Gemini CLI, Windsurf / Devin, Cline, Zed itself, OpenCode, Grok, Amp, Continue, LM Studio. Covered only by tests on scratch folders.
- **Windows and Linux:** path resolution only, by unit tests.
- **The wheel over the Settings window also reaches the canvas behind it:** scrolling Settings paused following once in this run. That predates Stage J; it's noted here, not fixed.
