# Connect any agent, not only Claude (Stage J)

**J0, 21 Oct 2026.** The plan for J1–J3, and what the code holds today.
- **The research:** how each MCP client adds a local server, read from its **current official documentation** (or its source, where marked) on **21 October 2026**.
- **The client research** was done in parallel by three research agents in this session, each told to quote only pages it read and to write "not documented" rather than infer. Their notes and the fetched pages are kept in the session's scratch folder; the URLs are below.
- **The installed CLIs:** what `codex`, `code`, `cursor` and `goose` print about themselves was read from their `--help` on this Mac.

## Why

Scrim's MCP endpoint is a standard streamable-HTTP server (`http://127.0.0.1:<port>/mcp`, C1). Any MCP client that can reach a local URL can use it. Today:
- Only Claude Code (C2) and Claude Desktop (F1) have connect flows.
- The AI tab says "Connected · claude-code" and "Following Claude".
- Every other agent needs a hand-written configuration.

## The clients (read 21 Oct 2026)

"[source]" marks a fact read from the client's own source code at the commit named, where the docs were silent. "Not documented" means exactly that: nothing is inferred.

### At a glance

| Client | How a server is added | Native local HTTP? | Token | List / remove | `clientInfo.name` |
|---|---|---|---|---|---|
| **Claude Code** (C2) | CLI `claude mcp add --transport http --scope user` | yes | `--header` | `claude mcp get/remove` | `claude-code` (seen live in Stage C: `docs/scrim-ports/settings/README.md`, "Connected · claude-code") |
| **Claude Desktop** (F1) | JSON file, stdio only | **no**: `mcp-remote` bridge | env in the entry | file | not found |
| **Codex CLI** | CLI `codex mcp add <name> --url <url>`, or TOML | yes (`url`) | `--bearer-token-env-var` (env name only) | `codex mcp list/get --json`, `remove` | `codex-mcp-client` [source] |
| **Gemini CLI** | CLI `gemini mcp add -s user --transport http`, or JSON | yes | `-H "Authorization: …"`, `${VAR}` in settings | `gemini mcp list`, `remove -s user` | `gemini-cli-mcp-client` [source] |
| **OpenCode** | JSON(C) file (`mcp`); the docs' CLI add is interactive | yes (`type: "remote"`) | `headers`, `{env:VAR}` | `opencode mcp list`; **no remove** (edit the file) | `opencode` [source] |
| **Cursor** | JSON file `~/.cursor/mcp.json`, UI, deeplink. **No documented CLI add** | yes (`url`) | `headers`, `${env:NAME}` | `agent mcp list`; remove in the UI or the file | not found (closed source) |
| **Windsurf**, now **Devin Desktop** | JSON file; **the docs give four different paths** | yes (`serverUrl` or `url`) | `headers`, `${env:VAR}` | the panel, or the file | not found |
| **Devin CLI** (Devin Desktop's default agent) | CLI `devin mcp add <name> --url <url> -s user` | yes | not documented for the CLI | `devin mcp list/get/remove` | not found |
| **VS Code** (Copilot) | CLI `code --add-mcp '<json>'` (user profile), URL handler, UI | yes (`type: "http"`) | `headers`, `${input:…}` / `${env:…}` | Command Palette, **MCP: List Servers** | `Visual Studio Code` (`productService.nameLong`) [source + the installed `product.json`] |
| **Cline** (extension and CLI) | CLI `cline mcp add <name> <url> --transport http --yes`, or the shared JSON file | yes (`type: "streamableHttp"`) | `headers` (`${env:}` in source only) | `cline config mcp --json`, `cline mcp remove` | `Cline` (extension); `@cline/core` (CLI SDK) [source] |
| **Zed** | UI, or `context_servers` in `settings.json` (JSONC) | yes (`url`) | `headers` (no env syntax documented) | the UI, or the file | `Zed` [source] |
| **Grok Build** (`grok` CLI) | CLI `grok mcp add --transport http <name> <url>`, or TOML | yes (`url`) | `--header`, `${VAR}` expanded, `bearer_token_env_var` | `grok mcp list --json`, `remove` | `grok-shell-<server name>` [source] |
| **grok.com Connectors**, **xAI API remote MCP** | cloud | **cannot reach 127.0.0.1** | — | — | — |
| **Goose** | `goose configure` (interactive), YAML file | yes (`type: streamable_http`, `uri`) | `headers` | `goose configure` | `goose-cli` / `goose-desktop` [source] |
| **Continue** | YAML `mcpServers`, or files in `.continue/mcpServers/` | yes (`type: streamable-http`) | `requestOptions.headers`, `${{ secrets.X }}` | edit the YAML | `continue-client` [source] |
| **Amp** | CLI `amp mcp add <name> <url>`, or `amp.mcpServers` in `settings.json(c)` | `url` (transport not documented) | `headers`, `${VAR}` | `amp mcp doctor`; local remove not documented | not found |
| **LM Studio** | in-app `mcp.json` editor (path not documented) | `url` (transport not documented) | `headers` | not documented | not found |

Roo Code is left out: its repository is archived (GitHub API, last push 15 May 2026).

### The details, with sources

**Codex CLI**
- **Docs:** https://learn.chatgpt.com/docs/extend/mcp?surface=cli (`developers.openai.com/codex/mcp` redirects there), https://learn.chatgpt.com/docs/config-file/config-reference, config-basic and config-advanced.
- **Source:** `openai/codex` at `rust-v0.163.0`.
- **The installed CLI:** 0.162.1 here prints the same usage: `codex mcp add [OPTIONS] <NAME> (--url <URL> | -- <COMMAND>...)`, `--url` ("URL for a streamable HTTP MCP server"), `--bearer-token-env-var <ENV_VAR>`, and `list --json`.
- **The file:** `~/.codex/config.toml`, or `$CODEX_HOME/config.toml` ("Codex stores its local state under CODEX_HOME (defaults to ~/.codex)"). It holds a `[mcp_servers.<name>]` table with `url = "http://localhost:3000/mcp"` (the docs' own local example). No Windows path is documented beyond `CODEX_HOME`.
- **Auth:**
  - `bearer_token_env_var`, `env_http_headers`, `http_headers` (static).
  - `codex mcp add` has **no `--header`** [source].
- **Remove:** `codex mcp remove <NAME>` [source; the docs show only `list`].
- **`clientInfo`:** `Implementation::new("codex-mcp-client", …).with_title("Codex")` [source: `codex-rs/codex-mcp/src/rmcp_client.rs`].

**Gemini CLI**
- **Docs:** https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md (= geminicli.com/docs/tools/mcp-server) and …/docs/reference/configuration.md. **Source:** commit `c1740d29`.
- **The command:** `gemini mcp add [options] <name> <commandOrUrl>`, with `-s, --scope` (user | project; **default "project"**), `-t, --transport http`, and `-H "Authorization: Bearer …"`.
- **The files:** `~/.gemini/settings.json` (user) and `.gemini/settings.json` (project). `GEMINI_CLI_HOME` moves the root.
- **The docs and the code disagree on the URL key:**
  - the docs document `httpUrl` for streamable HTTP and `url` for SSE;
  - the source marks `httpUrl` deprecated, and `gemini mcp add --transport http` writes `{url, type: "http", headers}`.
- **Conclusion:** use the CLI and let it write its own format.
- **Env references:** settings strings accept `$VAR`, `${VAR}` and `${VAR:-default}` (configuration.md).
- **List and remove:** `gemini mcp list`; `gemini mcp remove <name>` (scope default project). **`clientInfo`:** `gemini-cli-mcp-client` [source: `packages/core/src/tools/mcp-client.ts`].

**OpenCode** (`sst/opencode` now redirects to `anomalyco/opencode`)
- **Docs:** https://opencode.ai/docs/mcp-servers/, https://opencode.ai/docs/config/, https://opencode.ai/docs/cli/. **Source:** `v1.19.3`.
- **The file:** global `~/.config/opencode/opencode.json` (also `.jsonc`); `OPENCODE_CONFIG` names another file. **The Windows user path isn't documented** (the source uses XDG via `xdg-basedir`).
- **The entry:** `"mcp": {"<name>": {"type": "remote", "url": …, "enabled": true, "headers": {…}}}`. Env references are `{env:VAR}` ("replaced with an empty string" when unset). The format is JSON or **JSONC**.
- **The CLI:**
  - `opencode mcp add` is documented as an interactive guide; a non-interactive `--url` form exists only in source.
  - `opencode mcp list` exists. **There is no remove command**: edit the file.
- **`clientInfo`:** `opencode` [source].

**Cursor**
- **Docs:** https://cursor.com/docs/mcp, https://cursor.com/docs/mcp/install-links, https://cursor.com/docs/cli/mcp.
- **The files:** `~/.cursor/mcp.json` (global) and `.cursor/mcp.json` (project), merged. Per-OS paths aren't spelled out, only "your home directory".
- **The entry:** `{"mcpServers": {"server-name": {"url": "http://localhost:3000/mcp", "headers": {…}}}}` (the docs' own localhost example). `${env:NAME}` works in `url` and `headers`.
- **No documented CLI add.**
  - The installed `cursor` binary (3.25.9) does print VS Code's `--add-mcp`, but Cursor doesn't document it, so it isn't used.
  - `agent mcp list` lists servers. Removal is in Customize or the file.
- **JSON or JSONC:** not documented (one docs example starts with a `//` comment).

**Windsurf → Devin Desktop**
- **Docs:** `docs.windsurf.com/windsurf/cascade/mcp` redirects (307) to https://docs.devin.ai/desktop/cascade/mcp; also https://docs.devin.ai/desktop/devin-desktop-faq and https://docs.devin.ai/cli/extensibility/mcp/configuration.
- **The rename:** "On June 2, 2026, Windsurf is becoming Devin Desktop"; Cascade is now "the legacy Cascade agent".
- **Where the MCP file is, per Devin's own pages:**
  - `~/.config/devin/mcp_config.json`;
  - `~/.codeium/windsurf/mcp_config.json` ("the editor's MCP discovery");
  - `~/.codeium/mcp_config.json` (the FAQ);
  - `~/.codeium/<channel>/mcp_config.json`.

  They disagree, so **Scrim doesn't write it**. It shows the entry, `{"mcpServers": {"scrim": {"serverUrl": …, "headers": …}}}`, with Copy, and the pages.
- **The Devin CLI** (the default "Devin Local" agent) documents `devin mcp add <name> --url <URL>` with `-s project|user`, plus `list`, `get` and `remove`. It is offered through the CLI when `devin` is installed.

**VS Code with Copilot**
- **Docs:**
  - https://code.visualstudio.com/docs/agents/reference/mcp-configuration
  - https://code.visualstudio.com/docs/agent-customization/mcp-servers
  - https://code.visualstudio.com/docs/agents/run/manage-mcp-servers
- **The CLI:** `code --add-mcp '{"name":…,"type":…}'` "adds a server to the VS Code user-profile configuration … the VS Code format, not the portable Copilot user file". The installed `code` (3.25.9 build here) prints the option.
- **Two formats:**
  - VS Code's own: `servers`, in `.vscode/mcp.json` or the user profile, JSONC [source];
  - the portable Copilot one: `mcpServers`, in `~/.copilot/mcp-config.json` or a workspace `.mcp.json`.
- **The guided flow** labels the VS Code format "deprecated", but it remains supported, and it is what `--add-mcp` writes.
- **The user profile's `mcp.json` path isn't documented.** It sits beside `settings.json` [source], which is an inference.
- **The entry:** `{"type": "http", "url": …}`. `headers` takes `${input:…}` and `${env:…}`. Remove through **MCP: List Servers ▸ Uninstall**, or the file.
- **`clientInfo`:** `productService.nameLong`, which is "Visual Studio Code" in the installed build [source + the local `product.json`].

**Cline**
- **Docs:** https://docs.cline.bot/mcp/mcp-overview, https://docs.cline.bot/getting-started/config, https://docs.cline.bot/cli/cli-reference. **Source:** commit `e9dafbb`.
- **The CLI:** `cline mcp add docs https://example.com/mcp --transport http --header "Authorization: Bearer TOKEN" --yes`; `cline mcp remove <name>`; `cline config mcp --json`.
- **The file:** `~/.cline/data/settings/cline_mcp_settings.json` (or `CLINE_MCP_SETTINGS_PATH`), shared by the VS Code extension, the CLI and JetBrains [source].
- **The entry:** `{"type": "streamableHttp", "url": …, "headers": …}`. Omitting `type` defaults to the legacy SSE.
- **Env syntax:** none documented (`${env:VAR}` in source only), so a token would be written as-is.
- **`clientInfo`:** `Cline` (extension), `@cline/core` (CLI SDK) [source].

**Zed**
- **Docs:** https://zed.dev/docs/ai/mcp, https://zed.dev/docs/configuring-zed.
- **The file:** `context_servers` in `settings.json`, "JSON with support for `//` comments":
  - macOS and Linux: `~/.config/zed/settings.json` (Linux: or `$XDG_CONFIG_HOME/zed/`);
  - Windows: `%APPDATA%\Zed\settings.json`.
- **The entry:** `{"url": …, "headers": {"Authorization": "Bearer <token>"}}`. Env references aren't documented.
- **Adding and removing:** the UI only (no CLI). **`clientInfo`:** `Zed` [source].

**Grok**
- **Docs:** https://docs.x.ai/build/features/mcp-servers, https://docs.x.ai/build/settings, https://docs.x.ai/grok/connectors/custom-mcp-tunneling, https://docs.x.ai/developers/tools/remote-mcp. **Source:** `xai-org/grok-build` at `2bdd1d6`.
- **Grok Build is a local client:**
  - **Add:** `grok mcp add --transport http api https://mcp.example.com/mcp --header "Authorization: Bearer ${API_TOKEN}"`;
  - **List and remove:** `grok mcp list --json`, `grok mcp remove <name>`;
  - **The file:** `~/.grok/config.toml` (`%USERPROFILE%\.grok\config.toml`), or `$GROK_HOME`;
  - **Env references:** `${VAR}` is expanded in `headers`.
- **grok.com Connectors:** "Servers running on `localhost` or a private network address … are not directly reachable, and Grok will reject these URLs".
- **The xAI API's remote MCP tools** are called from xAI's servers.
- **`clientInfo`:** `grok-shell-<server name>` [source].

**Goose, Continue, Amp, LM Studio**
- **Goose** (https://goose-docs.ai/docs/guides/config-files; `block/goose` redirects to `aaif-goose/goose`):
  - YAML (`~/.config/goose/config.yaml`; Windows `%APPDATA%\Block\goose\config\config.yaml`), an `extensions` entry with `type: streamable_http` and `uri:`;
  - added through the interactive `goose configure`.
- **Continue** (https://docs.continue.dev/customize/deep-dives/mcp): YAML `mcpServers` with `type: streamable-http`.
- **Amp** (https://ampcode.com/docs/customize/mcp, https://ampcode.com/docs/cli/settings):
  - `amp mcp add <name> <url>`, or `"amp.mcpServers"` in `~/.config/amp/settings.json(c)`;
  - the transport a `url` uses isn't documented.
- **LM Studio** (https://lmstudio.ai/docs/app/mcp): an in-app editor whose file path isn't documented; "LM Studio currently follows Cursor's `mcp.json` notation".

## How C2 and F1 are built

Detailed in `docs/design/settings-and-connect.md` (C1–C2) and `docs/design/connect-claude-desktop.md` (F0–F2). In short:
- **Claude Code (C2), a CLI target.**
  - **The engine:** `cmd/mcp.rs` builds the argument list from the **live** address (`claude mcp add --transport http --scope user scrim <url> [--header …]`).
  - **The confirmation:** `mcp.run-connect` / `mcp.reregister` / `mcp.remove-old-entry` only request `dialog.*` UI, which an agent's commands never open.
  - **The window:** `connect_flow.rs` runs `claude` with that argument list on a background thread (`Services::run_program`; never a shell string). `mcp_registration.rs` reads `claude mcp get scrim` for "Registered address is out of date".
- **Claude Desktop (F1), a JSON-file target.** `cmd/claude_desktop.rs`:
  - the path per OS (documented, or marked inferred);
  - the entry (`mcp-remote@0.14.3`, pinned, with npx's full path);
  - the status from the file (Configured, Not configured, Out of date, Unreadable);
  - the safe write `apply`: invalid file refused, `mcpServers.<key>` spliced into the text (`claude_desktop/splice.rs`) and checked by parsing, a timestamped backup, a temporary file renamed over;
  - `apply` refuses inside an agent's `file_access` scope.

  `desktop_flow.rs` holds the panel and its three confirmations.

Both are good patterns, but each is a hand-built path: a third client would be a third module.

## The proposal: one client profile, as data

A client is a **profile**: one entry in `crates/engine/src/cmd/agent_clients/clients.json`, embedded in the binary. One engine module (`cmd/agent_clients.rs`) and one window module (`agent_clients_ui.rs`) serve every profile. **Adding a client is a new JSON entry, never a code path.**

```jsonc
{
  "id": "codex",                       // stable id (commands take it)
  "name": "Codex CLI",
  "docs": ["https://learn.chatgpt.com/docs/extend/mcp?surface=cli"],
  "verified": "2026-10-21",            // when the docs were read
  "verification": "docs-only",         // verified-live | docs-only | untested
  "client_names": ["codex-mcp-client"],// clientInfo.name values, for the friendly name
  "detect": {"programs": ["codex"], "dirs": ["{home}/.codex"]},
  "method": "cli",                     // cli | json-file | toml-file | manual
  "cli": {
    "program": "codex",
    "add": ["mcp", "add", "{key}", "--url", "{url}"],
    "add_token": ["--bearer-token-env-var", "{token_var}"],
    "remove": ["mcp", "remove", "{key}"],
    "status": {"args": ["mcp", "get", "{key}", "--json"], "url_at": "/transport/url"}
  },
  // json-file / toml-file instead: "file": {"paths": {"mac": "{home}/.cursor/mcp.json", "linux": …, "windows": …},
  //   "documented": {"mac": true, …}, "container": ["mcpServers"], "entry": {"url": "{url}"},
  //   "token": {"style": "env-ref", "header": "Bearer ${env:{token_var}}"} | {"style": "literal"} | {"style": "none"},
  //   "comments": true}
  // manual: "snippet": {"format": "yaml", "text": "…"}
  "bridge": false,                     // needs `npx -y mcp-remote@<pin>` (stdio-only clients)
  "cloud": false,                      // calls from a server: can't reach 127.0.0.1
  "notes": ["Codex reads the token from the environment variable you name; set it where Codex runs."]
}
```

- **Placeholders**, filled at run time only, never hard-coded:
  - `{url}`: the live address (`mcp::status`);
  - `{key}`: `scrim`;
  - `{token_var}`: `SCRIM_MCP_TOKEN`;
  - `{home}`, `{appdata}`, `{xdg_config}`.
- **The status**, from where the client keeps it:
  - the client's own read command when it has one with a URL in its output (`status.args`, read at `url_at`);
  - else the file, read through `file_access` (so agents can't read it, F1's rule).

  The states are Configured, Not configured, Out of date (another URL, or an old entry), and Can't read. **Never "Connected"** from a file or a list command. "Connected" is only what the endpoint itself sees: an `initialize` from that client.
- **The write** (file targets) generalises F1's `apply`:
  - the container path (`mcpServers`, `servers`, `context_servers`, `mcp`, or `amp.mcpServers` as one key);
  - a comment-aware splice for JSONC;
  - TOML through `toml_edit`, which keeps comments and ordering;
  - the same backup, temp-and-rename, and refusals.
- **Claude Code and Claude Desktop** get profiles too (`method: cli` and `json-file` with `bridge`), so they appear in the one list. Their existing flows stay as they are (verified live in C4 and F2). The profile describes them; the row opens their existing panel.

### The first profiles (and their labels before J3)

| Client | Method | Why | Label |
|---|---|---|---|
| Claude Code | cli (C2) | existing | verified live (Stage C, `docs/scrim-ports/settings/`) |
| Claude Desktop | json-file + bridge (F1) | existing | verified live through the bridge (F2); docs only for Desktop itself |
| Codex CLI | cli | documented add, `--url`, `get --json`, `remove`; `CODEX_HOME` allows a scratch run | docs only (until J3) |
| Gemini CLI | cli | documented add with `-s user`; the CLI writes its own (changing) format | docs only |
| Grok Build | cli | documented add, `${VAR}` headers, `list --json`, `remove` | docs only |
| Cline | cli when `cline` is installed, else json-file | both documented; the file is shared by the extension and the CLI | docs only |
| Devin CLI | cli | documented add, `-s user`, `list`/`get`/`remove` | docs only |
| VS Code | cli (`code --add-mcp`) | VS Code's own writer; the profile file's path is only inferred, so its status is too | docs only |
| Cursor | json-file `~/.cursor/mcp.json` | documented path and shape; no documented CLI add | docs only |
| OpenCode | json-file `~/.config/opencode/opencode.json` (JSONC) | documented path and shape; the CLI add is interactive; no remove command | docs only |
| Zed | json-file (JSONC, `context_servers`) | documented paths and shape; UI-only otherwise | docs only |
| Windsurf / Devin Desktop (Cascade) | manual (Copy) | the docs give four different paths | docs only |
| Goose | manual (YAML snippet) | the add is interactive, and the file is YAML | docs only |
| Continue | manual (YAML snippet) | YAML | docs only |
| Amp | cli | documented `amp mcp add <name> <url>` | docs only |
| LM Studio | manual | its file path isn't documented | docs only |
| grok.com, claude.ai, ChatGPT web connectors | — (cloud) | can't reach a loopback address | not supported (the generic card says why) |

## Claude-specific wording today (the audit)

| Where | Today | Is it really "the connected client"? | J1 |
|---|---|---|---|
| `ai_tab.rs:57` status row | "Connected · claude-code" (the raw `clientInfo.name`) | yes | "Connected · Claude Code" through the friendly-name table; the name verbatim when unknown; several clients listed |
| `ai_tab.rs:70` empty state | "Let Claude Code see and edit this canvas…" | yes (the general empty state) | agent-neutral, with the Connect list |
| `follow_agent.rs:155` `agent_name` | any name containing "claude" → "Claude"; other MCP clients verbatim; else "An agent" | yes | the table, verbatim, or "an agent"; never "Claude" for an unknown client |
| "Following {agent} · {artboard}" (AI tab), "{agent} is working on {artboard}" (the indicator's AccessKit name and `ui.inspect`) | from `agent_name` | yes | the client who made the latest edit |
| Activity feed rows | "Agent (mcp (http)): Rename" (the route) | the route, not the client | "Codex: Rename", from the client the edit came from (below); the route stays in `history.changes` |
| `settings_mcp.rs:78` | "Connect Claude Code again so it has the token" | the clients in general | "Connect your agents again so they have the token" |
| `mcp_ui.rs:96` | a client without a name is stored as "an MCP client" | yes | stored empty; shown as "an agent" |
| Diagnostics `mcpClient` | "{name} {version} connected" (raw, cleaned, 80 characters) | yes, verbatim | unchanged: raw names are what diagnostics should keep. "an agent" when empty |
| Undo labels | "Agent: …" | not Claude-specific | unchanged (the brief) |
| `ai_tab.rs:45`, `:74`; `settings_mcp.rs:106`; `mcp_registration.rs:83`; `cmd/mcp.rs` (`mcp.connect-claude-code` …); `agents::claude_add_*`; `ai.status`'s `connectCommand` | Claude Code's own flow | **no**: these really are Claude Code | kept; Claude Code becomes one row of the list |
| `desktop_flow.rs`, `cmd/claude_desktop.rs` | Claude Desktop's own flow | no | kept |

## Who made an edit (Stage I's source of truth)

- **Follow, the reveal and the dots** read `agents::Change.author.is_agent()`.
  - The author is set by the **route** (`Author::Agent("mcp (http)")`, `"control channel"`, …), never by `clientInfo`.
  - Nothing there depends on the client being Claude. Only the *name* shown comes from `clientInfo`, through `agent_name`.
- **The gap:** with several clients on the one HTTP endpoint, an edit doesn't say *which* client made it.
  - The endpoint has no sessions: one `Server` answers every connection.
  - `agent_name` takes the newest `initialize`, which can be the wrong client.
- **J1's fix:**
  - The endpoint issues an `Mcp-Session-Id` header on each `initialize` response. Streamable HTTP defines it, and clients send it back.
  - It remembers session → `clientInfo`.
  - Each tool call names its session's client in the control request, so the change log records **the client** beside the route (`Change.client`, additive; `history.changes` gains `client`).
  - A request without a session (a client that ignores the header) falls back to the newest `initialize`.
  - `is_agent()` stays the only test for "an agent's edit".

## Where I disagree with the brief, or read it a certain way (overrule any)

1. **Windsurf is Devin Desktop now, and its docs give four MCP paths.** "Do not infer paths" rules out writing it. Its row is manual: the entry, Copy, the documented pages. The **Devin CLI** gets its own `cli` row.
2. **VS Code:** the documented CLI (`code --add-mcp`) writes the user-profile `mcp.json`, which VS Code's guided flow now calls deprecated in favour of `~/.copilot/mcp-config.json`.
   - I use the CLI: it is VS Code's own writer and supported.
   - The status reads the profile file at the path VS Code's source uses, labelled "inferred" (not documented).
3. **YAML clients (Goose, Continue) are manual.** The brief covers JSON and TOML writers. A YAML writer that keeps comments is a dependency with no other use here. They get a YAML snippet to Copy and the documented steps.
4. **JSONC files** (Zed, OpenCode, VS Code's profile, Amp):
   - Edited by a comment-aware splice that changes only Scrim's entry, checked by parsing the result with comments stripped.
   - When the splice can't be sure, **nothing is written** (Copy only). A JSONC file is never pretty-printed over its comments.
5. **Tokens:**
   - **Env-var references** wherever the client documents them: Codex `--bearer-token-env-var`, Cursor and Windsurf `${env:}`, Gemini and Grok and Amp `${VAR}`, OpenCode `{env:}`. The variable is `SCRIM_MCP_TOKEN`.
   - **The catch:** the person must set that variable where the client runs. That is easy for CLIs and harder for apps started from the Finder; the panel says so.
   - **Clients without a documented reference (Cline, Zed):** a warning and an explicit confirmation before a token is written into their file.
6. **Claude Code and Claude Desktop keep their own flows** (verified live) and join the list as rows that open them. They aren't rebuilt on the profile engine in this stage: that would be a rewrite with no visible gain, and C4/F2's evidence would no longer cover the new code.
7. **Cursor's installed binary has `--add-mcp`**, but Cursor doesn't document it, so it isn't used. Its documented file is.
8. **"Connected"** stays reserved for a client the endpoint has seen initialize. A configured client that hasn't started yet is "Configured".
9. **Session ids** are added to the endpoint (J1), so that "the most recently active client" means the one that made the latest edit, not the latest to initialize.

## New dependency (J2)

`toml_edit` (MIT OR Apache-2.0; already in `Cargo.lock` through `toml`) becomes a direct dependency of `figcraft-engine`, for writing Codex's and Grok's `config.toml` with comments and ordering kept. It is the format-preserving editor the brief names. Codex and Grok are written through their own CLIs when present; the file writer is the fallback when the CLI isn't installed.

## J1 as built: agent-neutral names

- **Sessions on the endpoint** (`crates/mcp`):
  - **Issuing them:** `Server::handle_session` is the HTTP transport's entry. An `initialize` issues an `Mcp-Session-Id` (64 hex digits, from `settings::new_token`), sent in the response header and remembered with its `clientInfo` (the last 32).
  - **Attributing requests:** before every message, the backend is told which client it comes from (`Backend::set_client`): the session's, or the newest to initialize for a request without a known session. Lenient: a client that ignores the header, or one carrying a session from before a restart, is not refused.
  - **stdio** (`handle_line`) issues no session.
- **The client on each edit:** `ControlRequest.client` (the app's backend sets it) → `Agents.client` for the request → `Change.client` in the change log, for agents' edits only. `history.changes` gains `client`. `Author::is_agent()` is unchanged, and still the only test of "an agent's edit".
- **Names** (`agents::client_display_name`):
  - the friendly name when the table knows it: `claude-code` → Claude Code, `codex-mcp-client` → Codex, `gemini-cli-mcp-client` → Gemini CLI, `opencode`, Visual Studio Code / Code - OSS → VS Code, Cline / `@cline/core`, Zed, `goose-cli` / `goose-desktop` → Goose, `continue-client` → Continue, `grok-shell-…` → Grok;
  - else the name as announced (control characters out, cut at 32 characters with "…");
  - `None` for no name, which reads "an agent" (or "An agent" at the start of a sentence). Never "Claude" for another client.
- **Where it shows:**
  - **The AI tab:** the header is "Agents". The status row is "Connected · Claude Code, Codex": every client connected in the last 10 minutes, newest first (`mcp.status` gains `clients`, each with `display`). The empty state is agent-neutral.
  - **Following and the indicator:** "Following Codex · Poster"; "Codex is working on Poster" (the indicator's AccessKit name).
  - **Activity rows:** "Codex: Create frame" when the client is known, else the route as before.
  - **Settings ▸ MCP:** the token note says "Connect your agents again".
  - **Diagnostics:** "<name as announced> <version> connected", or "an agent … connected".
  - **Undo labels:** "Agent: …", unchanged.
  - **"Most recently active"** is the client of the latest edit; the status row lists all.
- **Tests:**
  - `agents_tests.rs`: the name rules, and the change log's client;
  - `crates/mcp/tests/sessions.rs`: attribution per session, the fallback, nameless clients, stdio, and the real header;
  - `client_names_tests.rs`: seven names across every place above, and identical follow, reveal and dots for each, with several clients the latest to edit named.
  - Making the window ignore the edit's client fails the several-clients test.

## J2 as built: Connect other agents

- **The profiles** are data: `crates/engine/src/cmd/agent_clients/clients.json` (16 clients), embedded at build time. Adding a client is a JSON entry: its docs, the date they were read, its label, how it's found, its command, its file (format, paths per OS, whether the docs name them, the servers' key path, the entry, how a token goes in, whether Scrim may write it) or its snippet. Claude Code and Claude Desktop are `builtin` rows that keep their own flows (C2, F1).
- **The engine** (`cmd/agent_clients.rs`, `agent_clients/jsonc.rs`, `agent_clients/tomlfile.rs`):
  - `report` is everything about one client for this window: the plan (the command, the file and entry, the snippet), the status from its file, the notes. Files are read only where the desktop app says (`McpEndpoint::client_env`, set from `Services::agent_client_env`); tests, the headless app and the browser read none. Inside an agent's request a file reads as unreadable (through `file_access`).
  - `changed` makes one change in a file's text: TOML through `toml_edit` (comments and order kept); JSON and JSONC by a splice that changes only Scrim's member and is checked by parsing the result. A JSONC file whose comments can't be kept is refused, never pretty-printed over.
  - `apply` is F1's safe write: refused inside an agent's request or for a read-only profile, a backup named `<file>.scrim-backup-<time>`, a temporary file, then a rename.
  - Commands: `mcp.agent-clients` and `mcp.client-status` (read-only, on the MCP routes), `mcp.connect-client` (shows the panel), `mcp.copy-client-entry`, `mcp.copy-agent-snippet`, and the confirm-only `mcp.write-client`, `mcp.reregister-client`, `mcp.remove-old-client-entry`, which refuse an agent and only ask the person.
- **The window** (`crates/ui-egui/src/agent_clients_ui.rs`, shared by the AI tab and Settings ▸ MCP):
  - **How a client is changed:** its own command when the program is found (Codex, Gemini CLI, Grok, Cline, Devin, Amp, VS Code's `--add-mcp`); else its file when the profile allows writing and the docs name the path on this system; else Copy. Re-register through a CLI is its remove, then its add; a CLI without a remove command falls back to the file or Copy.
  - **The rows:** found clients first (a program on the `PATH`, or the configuration folder in the desktop app; looked for every ten seconds, nothing run); the rest under More; the card for any other agent behind Other agent.
  - **The panel:** the exact command lines or the file path and entry, the profile's notes, the token note (the env-var to set, or a warning that the token is written in plain text), the state's note, Run / Write / Re-register, Copy, Documentation, Close, the outcome, and Remove the old entry.
  - **The confirmations** read the file again, aren't shown when there's nothing to do, show the command or the path, say that Scrim doesn't restart the client, and for a plain token say so and are answered with "Write the token".
- **Not built:** reading a CLI's own list command for the status (`status.args` in the J0 sketch); the files carry the status for every CLI client here. YAML clients stay Copy only.

## J3 as built: verification

- **Live, on a scratch home** (`docs/scrim-ports/connect-agents/README.md`): the release build's real Connect for **Codex**, clicked in the window, with Codex's own `mcp list --json` and `mcp get` before and after: add, Re-register (remove, then add), and Re-register with authentication on (`--bearer-token-env-var`). Codex is now labelled **verified live**; every other client stays **docs only**.
- **File clients** (Cursor, Zed): written and refused live on scratch files; the apps weren't started.
- **Fixes from the live run:** the row and button layout, the token masked in everything the window paints, the "address but not the token" wording, the panel's intro only when Scrim will act, and Stage I's Resume button inside a scrolling AI tab.
- **Not verified:** a Codex session using the tools (it needs the owner's sign-in); VS Code's `--add-mcp` (it talks to the running VS Code); the GUI clients reading what was written; the clients that aren't installed; Windows and Linux beyond path resolution.
