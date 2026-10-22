# The Follow agents setting (Fix V, 22 Oct 2026)

**Why:** after installing a Fix T/U build, the person found Follow agents off. They hadn't switched it off. Earlier that day it was on: following paused and resumed. With the new build, the first agent batch logged "agent layers: 0 entered, 7 drawn final at once (off screen, follow off: 7)", and no following event appeared for the whole build.

## V1: the root cause

**In one sentence:** `camera.follow-agents` is the only thing that turns following off, any MCP client may run it, and run without `on` it toggles; an agent's call leaves no trace beyond "connected" and "working: begin". No migration, no settings change and no button touched the saved value.

### The reproduction

`docs/scrim-ports/follow-setting/repro.py` starts the app built from this tree once per case. Each case has its own scratch HOME and scratch settings folder, MCP on 7690 and the control channel on 7979. The script:
1. reads following's state from `ui.inspect`: the engine's (`agent.follow`) and the AI panel header's button (`agent.aiPanel.header.follow`);
2. quits the app;
3. reads `follow_agents` back from the settings file the app saved.

The settings files are real (`apps/figcraft/tests/fixtures/settings/`):
- `state-v7-before-fix-t.json`, written by the build just before Fix T: `settings_version` 7, Follow on, an unmarked 30 s grace;
- `state-v8-fix-t.json`, written by a Fix T build.

Only Settings ▸ Diagnostics' scratch entries were removed. Earlier versions are the first file with the keys added after them removed.

The output is in `repro-out.txt`:

| Case | Engine | Header button | Saved |
|---|---|---|---|
| Fresh folder (only the MCP port set) | following | following | on |
| Before Fix T (v7, on, grace 30 unmarked) | following | following | on, now version 8 |
| A Fix T file (v8, on) | following | following | on |
| Versions 0, 1 and 2 (no `follow_agents` key yet) | following | following | on |
| Versions 3 to 7, on | following | following | on |
| Versions 3 to 7, off | off | off | off |
| **An agent runs `camera.follow-agents {}`** (`run_command`) | **off** | **off** | **off** |
| An agent runs `camera.follow-agents {on: false}` | off | off | off |
| An agent lists commands with filter "agent", then runs it with no `on` | off | off | off |
| An agent runs it through `batch` | off | off | off |

In every agent case Diagnostics kept only "repro-agent 1.0 connected" and "working: begin … first request". Nothing said what the agent ran.

### Each suspect

- **(a) A migration or the version bump: false.**
  - `Settings::sanitized` (T1, U1 and every earlier step) never touches `follow_agents`.
  - The key has read with `#[serde(default = "yes")]` since version 3, so a file without it reads on.
  - A file that fails to parse whole falls back to the defaults, which are also on.
  - Every version from 0 to 8 kept its value, on or off, through a load, the migration to version 8 and the save.
- **(b) The T4 header button or the moved Settings toggle: false.**
  - Both run `camera.follow-agents` and read `follow_state`, from the same field, `prefs.settings.follow_agents`. The button showed what the engine had in every case.
  - The Settings checkbox reports a click, never its value (`widgets::checkbox` returns `clicked()`), so drawing the page changes nothing.
  - A probe showed the button doesn't keep the keyboard after a click: Space (the canvas's pan key) and Enter then changed nothing.
  - A click on the crosshair does turn following off, by design. It isn't logged, so a person's misclick can't be ruled out from the code either. See "What can't be settled" below.
- **(c) An agent-callable command: TRUE.**
  - `camera.follow-agents` is classified "Write, outside the undo history" (`cmd/access.rs`, `command-access.md`). Any MCP client allowed to write can run it through `run_command` or `batch`, and so can the CLI and the control channel.
  - It has no person-only guard, unlike `pref.agentGrace` (Fix Q) and the chat commands (R5).
  - Without `on` it toggles. An agent that wants following on and calls it with no `on` turns it off.
  - `list_commands` with filter "agent" shows it to agents as "camera.follow-agents — Follow agent {on? (default: toggle; …)}".
  - Stage I3 chose this ("reachable from MCP like every command"), and `crates/mcp/tests/follow_agent.rs` asserted that an agent's `{}` turns it off.
  - No other command, tool or migration writes the field: `grep` finds one writer, `cmd/camera.rs` › `follow_agents`.
- **(d) Something else: nothing found.** There was no key rename and no serde default other than "on". The agents-can't guard isn't applied to the person's own change. What made this a mystery is that **no change of the setting is logged**: not a person's, not an agent's. The header's off state also looks almost like an idle on state: no accent, same icon.

### The failing sequence

1. Following is on, saved in `state.json` as `follow_agents: true`, and it survives the T1/U1 migration (shown above).
2. An MCP client connected to the person's Scrim (Diagnostics: "<client> connected"; the person's log shows Codex clients connecting in threes, and Claude Desktop's agent did the build) calls `run_command {command: "camera.follow-agents", params: {}}`. Or it calls it with `on: false`, or inside `batch`.
3. The engine sets `prefs.settings.follow_agents = false` and answers `{on: false, state: "off"}`. The window saves Settings.
4. Diagnostics shows "connected" and a "working: begin", nothing more. The header's crosshair loses its accent.
5. The next agent batch lands off screen. The reveal says "drawn final at once (off screen, follow off: 7)". No following event follows, because following is off.

### What can't be settled from here

- **Which client made the call on the person's computer:** successful agent commands aren't logged, and their settings may not be read (the brief's rule).
- **Whether the person clicked the crosshair:** a person's click isn't logged either, so that route can't be excluded from the code alone. The person says they didn't, and it is the only other route.

Fix V3 logs both from now on, with the origin.

## V2: the fix

**What changed:**
- **Following is the person's alone.** `camera.follow-agents` refuses an agent on every route (MCP's `run_command` and `batch`, the CLI, the control channel, plugins), whatever it sends. The refusal reads "only the person turns following on or off: the Follow agents button in the AI panel, or Settings ▸ Agents".
- **Each refusal is noted** in Settings ▸ Diagnostics: "follow setting: stays ON (refused: an agent asked, over MCP, session 3f1520b6…)". The route is in fixed words and the session in 8 letters and digits. Nothing the agent sent, nor a name a client or plugin gave, goes in (`Entry::FollowRefused`).
- **One source of truth:** `prefs.settings.follow_agents`, read through `follow_state` by the camera, the reveal, the header button and now the Settings toggle too, which read the field directly before.
- **Settings are kept as they change** (a second defect V2's restart test found):
  - **Before:** a change of Settings (Follow agents among them, and every Diagnostics line) was written only when the app quit, or when a plugin, an online token or a panel width changed. A kill or a crash lost it.
  - **Now:** the window hands the settings to `Services::keep_prefs` every frame. The desktop app writes them, atomically as before, whenever they differ from what it last wrote.
  - **Why this isn't the root cause:** a lost write can only undo a change, never turn on into off by itself. The 11:43 pause events were saved, and following was saved with them.
- **No marker for following:** no migration touches it, and a test now fails if a migration changes any key but its own. A marker like the grace's would only be needed if a migration ever had to move an unchosen value.
- **A fresh install follows agents,** as before (`follow_agents: true` by default, and `serde(default = "yes")` when the key is missing).

**Tests:**

| Test | Without the fix |
|---|---|
| `camera_tests.rs` › `an_agent_cant_change_following_and_the_refusal_is_noted`: four routes, three forms and a batch, each refused, the setting unchanged, the line noted, no plugin-given name | fails (the agent turns it off) |
| `crates/mcp/tests/follow_agent.rs`: `run_command` with `{}`, `on: false`, `on: true`, and `batch`, refused in plain words over MCP | fails (it asserted the toggle before) |
| `apps/figcraft` › `follow_setting_tests.rs` › `the_persons_change_is_kept_across_a_restart`: off, killed, still off; on, quit, still on | fails ("off after a kill": it came back on) |
| `follow_setting_tests.rs` › `a_file_from_the_build_before_fix_t_with_follow_on_stays_on`, the real files, start, quit, start | passes before too: (a) was false |
| `follow_setting_tests.rs` › `every_supported_settings_version_keeps_follow_as_it_was`, versions 0 to 8, on and off | passes before too |
| `follow_setting_tests.rs` › `a_fresh_settings_folder_follows_agents` | passes before too |
| `follow_setting_tests.rs` › `a_migration_changes_nothing_but_its_own_keys`: only `settings_version` and the unmarked grace change | passes before too; it fails for any future migration that writes over another value |
| `ai_panel_tests.rs` › `the_follow_button_and_the_settings_toggle_always_agree_and_survive_a_restart` | passes before too: (b) was false |

**A flake found on the way:**
- **The symptom:** the first V2 CI run failed `recovery_tests.rs` › `a_lock_freed_a_moment_later_is_not_a_run_that_is_still_going`, about 1 run in 4 with V2's tests and 0 in 8 without them.
- **The cause:** the version sweep started the app's services 34 times. Each start forks `defaults read` for the system's language and motion setting, and a forked child holds a copy of the recovery test's lock until it execs (the #334 situation that test is about).
- **The fix:** the sweep now reads and writes through `read_prefs` (taken out of `Desktop::load_from`, which uses it) and `write_prefs`, without starting the services. The suite then passed 10 runs in 10.

**Decisions (overrule any):**
1. **The CLI, the control channel and plugins count as agents here,** as they do for the chat and grace commands. A script that wants following off now needs the person.
2. **The refusal line names the route and the session, not the client.** The client's name is the agent's own words; the session ties the line to that client's "connected" and "working" lines.
3. **`pref.revealAgentWork`, `pref.workingIndicator` and `pref.revealSpeed` stay open to agents,** as the brief names only Follow. The same mystery could happen to them. Making them the person's too is a one-line change each.
4. **All Settings are now written as they change,** not only Follow. That is the smallest rule that makes "a human change persists across a restart" hold. The file is written at most once a frame, and only when something in it changed.
