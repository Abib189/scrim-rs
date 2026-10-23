# Settings access: who can change a saved setting (Fix W2, 23 Oct 2026)

**Why:** Fix V found that an agent could turn Follow agents off, and nothing said so. Fix V2 closed that one command. This page lists every command that writes a saved setting or a persistent preference, and whether an agent can change it. Fix W3 makes the rule general.

## How the list was made

- **Read from the code:** every place the engine writes `Session::prefs` (`grep` over `crates/engine/src`), each mapped to the command that runs it.
- **Checked by running every command:** a probe (`crates/engine/src/cmd/settings_probe_tests.rs` during the audit; W3 keeps it as a test) ran all registered commands, except the file and import ones that need real paths. It used about 45 kinds of parameters, as the person and then as an agent, and compared the saved preferences before and after. It found every command in the first table that a generic parameter can reach. The toolbar's move and group, the invite links and the plugin installs need parameters it doesn't guess, so they come from the code.
- **"An agent" means every route:** MCP's `run_command` and `batch`, the CLI, the control channel and plugins all run as an agent (`Author::Agent`). MCP also holds the commands to Settings ▸ Tool access, which allows writes by default.
- **Counts:** 58 commands write a setting; 50 of them could be changed by an agent before Fix W3.

## Commands that write a saved setting

| Command | Saved setting it writes | Can an agent change it today (before Fix W3)? |
|---|---|---|
| `camera.follow-agents` | `follow_agents`, `follow_chosen` | no: refused on every route (Fix V2) |
| `pref.accent` | `accent` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.agentGrace` | `agent_grace_secs`, `agent_grace_chosen` | no: refused on every route (Fix Q1) |
| `pref.autoSave` | `auto_save` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.chatTimeLimit` | `chat_time_limit_mins` | no: refused on every route (R5) |
| `pref.describeChange` | `describe_change` | no: refused on every route (R5) |
| `pref.highlightHover` | `highlight_on_hover` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.keepTool` | `keep_tool` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.language` | `language` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.motion` | `motion`, `reduce_motion` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.nudge` | `nudge_small`, `nudge_big` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.numberKeysOpacity` | `number_keys_opacity` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.preserveUnsaved` | `preserve_unsaved` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.publishServer` | `publish_server` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.reduceMotion` | `reduce_motion`, `motion` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.renameDuplicates` | `rename_duplicates` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.revealAgentWork` | `reveal_agent_work` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.revealSpeed` | `reveal_speed` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.showDimensions` | `show_dimensions` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.theme` | `theme` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.userName` | `user_name` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `pref.workingIndicator` | `working_indicator` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.themeDark` | `theme` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.themeLight` | `theme` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.themeSystem` | `theme` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.pixelGrid` | `view` (kept for new files) | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.layoutGrids` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.rulers` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.slices` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.comments` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.multiplayerCursors` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.outlinesHiddenLayers` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.outlinesObjectBounds` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.maskOutlines` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.frameOutlines` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.snapGeometry` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.snapObjects` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `view.snapPixel` | `view` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `toolbar.toggle` | `toolbar` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `toolbar.move` | `toolbar` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `toolbar.group` | `toolbar` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `toolbar.ungroup` | `toolbar` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `toolbar.reset` | `toolbar` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `mcp.set-port` | `mcp.port`, `mcp.port_chosen` | no: refused on every route (C1) |
| `mcp.toggle-auth` | `mcp.require_auth`, `mcp.token` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `mcp.set-root-dir` | `mcp.root_dir` | no: refused on every route (C1) |
| `mcp.set-tool` | `mcp.disabled_tools` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins; turning a tool on is refused (C1.3) |
| `mcp.set-tool-group` | `mcp.disabled_tools` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins; turning tools on is refused |
| `mcp.restore-tools` | `mcp.disabled_tools` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins; refused when it would turn tools on |
| `diagnostics.retention` | `diagnostics_days` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `diagnostics.clear` | `diagnostics` (the saved log) | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `fonts.muteMissing` | `fonts_muted` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `chat.allow-agent` | `chat_allowed` | no: refused on every route (R5) |
| `chat.forget-agents` | `chat_allowed` | no: refused on every route (R5) |
| `collab.registerLinks` | `invite_links` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `plugin.import` | `plugins` (installs a plugin) | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `plugin.remove` | `plugins`, `plugin_settings` | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |
| `plugin.directory.install` | `plugins` (installs a plugin) | yes: MCP (`run_command`, `batch`), the CLI, the control channel, plugins |

`pref.appearance`, `toolbar.state`, `mcp.tools`, `mcp.status` and `diagnostics.events` read only.

## Writes that aren't settings (window state and records)

These change the saved preferences file too, but they are the window's memory or a record, not a choice the person makes in Settings. They stay open to agents.

| Command | What it writes |
|---|---|
| panel.mode.*, panel.tab.* | `panel` (the right panel's last mode and tab) |
| help.gettingStarted | `getting_started_dismissed` |
| file.open, file.save, file.saveAs, file.openRecent, file.close… | `recent_files` |
| collab.* (online files) | `online_files` (owner tokens of files put online) |
| plugin.directory.list, .publish, .unpublish | `plugin_directory` (the server browsed last) |
| a plugin's own `setting_set` | `plugin_settings` (the plugin's storage) |
| the window (not commands) | `panel_widths`, `pages_height`, `home_views`, `home_sort`, `home_order`, `recent_actions` |
| every agent request, the window | `diagnostics` (new lines) |

## What isn't in scope

- **Commands that write outside the settings file** are another kind of change: `mcp.connect-claude-code`, `mcp.reregister`, `mcp.remove-old-entry` and the client connectors write other apps' configuration files, and `mcp.restart-server` restarts the endpoint. Their access is `command-access.md`'s.
