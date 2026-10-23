# Settings access: who can change a saved setting (Fix W2, 23 Oct 2026)

**Why:** Fix V found that an agent could turn Follow agents off, and nothing said so. Fix V2 closed that one command. This page lists every command that writes a saved setting or a persistent preference, and whether an agent can change it. Fix W3 makes the rule general.

## How the list was made

- **Read from the code:** every place the engine writes `Session::prefs` (`grep` over `crates/engine/src`), each mapped to the command that runs it.
- **Checked by running every command:** a probe (`crates/engine/src/cmd/settings_access_tests.rs` › `probe`, run by hand; W3's `every_command_that_writes_a_setting_is_marked` runs the same sweep in CI) ran all registered commands, except the file and import ones that need real paths. It used about 45 kinds of parameters, as the person and then as an agent, and compared the saved preferences before and after. It found every command in the first table that a generic parameter can reach. The toolbar's move and group, the invite links and the plugin installs need parameters it doesn't guess, so they come from the code.
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

## Fix W3: default-deny for settings

**The rule:** "writes a saved setting" is a property of the command in the registry (`CommandSpec::setting`, set from `access::SETTINGS`, the first table above).
- **Before any command runs** (`Session::execute`), `access::gate` refuses an agent a marked command on every route: MCP's `run_command` and `batch`, the CLI, the control channel, plugins. It doesn't matter what the agent sends.
- **The refusal:** "only the person changes Scrim's saved settings: in Settings, or by running the command themselves".
- **Settings ▸ Diagnostics notes it** (kind `setting`): "setting pref.theme: unchanged (refused: an agent asked, over MCP, session 3f1520b6…)". It says the registered command's id, the route in fixed words and the session's first 8 letters and digits. Nothing the agent sent goes in: no parameter, value or client name.
- **Following keeps Fix V2's own refusal and line** ("follow setting: stays ON (refused: …)"), in its command.

**The allowlist** (`access::AGENT_SETTINGS`), each entry with its reason in the registry:

| Command | When an agent may run it | Why |
|---|---|---|
| `agent.finish-working` | always | it changes no setting: it tells the window the agent is done |
| `mcp.set-tool` | only with `on: false` | it can only take an agent's own access away, never give it |
| `pref.agentGrace` | only with the test hooks on (`Session::enable_test_hooks`) | tests and live checks set a grace for that session; it is never saved |

**It can't regress:** `settings_access_tests.rs` › `every_command_that_writes_a_setting_is_marked` runs every registered command as the person with about 30 kinds of parameters, and fails for any command that changed a saved setting without being in `access::SETTINGS`. Leaving `view.rulers` out of the list fails it, naming the command and the setting.

**Tests:**
- `settings_access_tests.rs`:
  - `every_command_that_writes_a_setting_is_marked`, which also checks that the list holds only registered, marked commands and that the allowlist is short, real and reasoned;
  - `every_setting_command_is_refused_to_an_agent_on_every_route_and_noted`: 57 commands × 5 routes × 3 parameter sets refused, the settings unchanged, the line noted with nothing the agent sent, and `command.batch` too;
  - `the_allowlist_lets_an_agent_finish_and_turn_a_tool_off_and_nothing_more`.
- `chat_route_tests.rs` › `the_control_channel_and_the_mcp_route_are_refused_every_setting_command`: the window's real routes.
- `crates/mcp/tests/follow_agent.rs` and `panel.rs`: over MCP, the reveal, its speed, the dots, the grace, the theme and the accent are refused, and reading the appearance still works.
- **Tests that changed with the rule:**
  - the grace, chat, port, MCP tool and diagnostics tests now expect the shared refusal;
  - two UI tests flip the reveal and the dots as the person;
  - an agent can no longer turn on a tool that is already on.

**Decisions (overrule any):**
1. **The property is filled from one list** (`access::SETTINGS`) by the registry, not written into each of the 58 specs. The list sits next to Tool access's own classification, and the sweep test keeps it complete.
2. **There is no runtime backstop** that would catch an unmarked command writing a setting while an agent runs it. The post-command hook can't restore preferences, and nested batches make its snapshot unreliable. The sweep test is the guard instead. Its limit: a writer only reachable with parameters it doesn't guess (the toolbar's move and group, invite links, plugin installs) must be listed by hand, as these are.
3. **One refusal for every setting command:** the chat, grace and port commands' own words no longer show to agents, because the gate runs first. Their own guards stay as a second check.
4. **`mcp.set-tool-group` and `mcp.restore-tools` are refused to agents outright,** turning off included. The allowlist names `mcp.set-tool` only, as the brief asks.
5. **The kept View toggles (rulers, grids, snapping, outlines…) count as settings:** they write the default for new files. On the open file they could be left to agents in a later change, if wanted.
6. **The test hook for the grace is on the allowlist:** without it the live checks can't shorten the grace. It is never saved.

## Fix W4: every setting change is noted with who made it

`crates/engine/src/settings_changes.rs` (`settings::changes`). Each saved setting a command or a migration changes gets one line in Settings ▸ Diagnostics (kind `setting`):

| Line | Made by |
|---|---|
| "setting reveal_speed: normal → slow (made by the person's Settings)" | a control in Settings: the window marks the time it draws Settings (`agents.from_settings`) |
| "setting theme: system → dark (made by the person's command)" | the command another way: a menu, the Actions palette, a shortcut |
| "setting agent_grace_secs: 30 → 60 (made by a settings migration)" | `Settings::sanitized` as the app starts, a value put back in range included |
| "setting pref.theme: unchanged (refused: an agent asked, over MCP, session …)" | an agent (Fix W3) |

- **How:** `access::gate` takes the settings before the person's setting command, and `note_change` (after it succeeded) notes each one that differs. `sanitized` compares the file as read with the result.
- **Values:**
  - a short plain value is shown ("system → dark", "7 → 1", "none → true");
  - who the person is (`user_name`), where they publish (`publish_server`), a token, a path, and a list (fonts, plugins, the toolbar, agents allowed to run) read "changed".
- **Not said here:** following (its own lines, Fix V3) and the markers that a value was chosen (`*_chosen`), which go with their value.
- **Nothing is collapsed:** these are rare.
- **An agent can't pose as the person:** it is refused before anything runs, whatever it sends (an `origin`, a flag) and even while the window draws Settings. Only the window sets `from_settings`, and only around drawing the Settings pane.

**Tests:**
- `settings_changes_tests.rs` › `each_origin_of_a_setting_change_is_noted`:
  - the person's command;
  - the person's Settings, for a preference and a View toggle;
  - a name and a token never shown;
  - no line when nothing changed;
  - a migration;
  - following's own line, not doubled.
- `settings_changes_tests.rs` › `an_agent_cant_pose_as_the_person`: an agent sending `origin` or `from_settings`, while the window draws Settings, is refused, and nothing is noted as the person's.
- `ai_panel_tests.rs` › `a_setting_changed_in_settings_is_noted_as_the_persons_settings`: a real click on Settings ▸ Agents' "Show working indicator", then the command. It fails without the window's flag.
- **Updated:** `diagnostics_tests.rs` › `retention_drops_old_events_and_zero_keeps_none` uses the real clock, because changing the retention is now itself noted.

**Decisions (overrule any):**
1. **The setting's name is its key in the settings file** (`reveal_speed`, `mcp.port`, `view.rulers`), not its label in Settings. The key doesn't change with the language.
2. **The chosen markers aren't said:** "setting agent_grace_chosen: false → true" would double every grace line.
3. **A value put back in range at startup is noted as a settings migration:** the person didn't make it, and the line says so.

## Fix W6: live

`docs/scrim-ports/follow-setting/w6/`: two launches of this tree's debug build on scratch settings, starting from the real pre-Fix-V file with following saved off and no marker.
- **Following:** on after the first launch, with the migration's line; nothing more on the second.
- **The stub client named as Codex:** refused 8 of 8 setting changes, each noted, and every setting stayed as it was.
- **The allowlist:** `finish_working` and turning a tool off worked; turning it on was refused.
- **The build:** a batch of 40 layers 3,000 units away, centred to 0.00 px, 41 entered.
- **The dots:** on in every capture from the first frame drawn until `finish_working` (30 of 31 counting one taken 0.03 s after the build began).
- **The screen was unlocked.**
