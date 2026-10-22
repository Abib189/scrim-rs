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
