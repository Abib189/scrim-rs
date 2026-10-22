# One working state, and a slower reveal (Stage I.1)

**IA, 21 Oct 2026.** What the code does today, read from it, and the plan for IB–IE.

**The report:** an agent built a poster in six steps, 2–3 s apart, because it thinks between tool calls. The dots above the artboard disappeared between steps, and the reveal was too fast to follow.

## How "working" is decided today

**The cause is confirmed.** `working_indicator::alpha` fades the dots out once the agent has been idle `IDLE` (0.6 s) since `last`. `last` moves only when an agent's *edit* lands (`working_indicator::tick`, on `landed.iter().any(|l| l.agent)`). Reads (screenshots, `inspect_document`, `lint_design`) never move it. So every pause over 0.6 s + 0.4 s ends the indicator, and the next edit fades it in again.

**Everything that reads some notion of "an agent is working"**, each from a different source:

| Where | Reads | Source today |
|---|---|---|
| The dots (`working_indicator::paint`) | `alpha(began, last)` | the last agent edit, 0.6 s idle |
| The outline and its pulse (same function) | the same `alpha` | the same |
| The AccessKit status ("{agent} is working on {artboard}", `working_indicator::accessibility`) | drawn only while the row is drawn | the same |
| `ui.inspect` › `agent.working` (`follow_agent::working_text`) | the indicator's artboard | the same |
| The load budget's "something animates" (`follow_agent::budget`) | `indicator.artboard().is_some()` | the same |
| Follow's status, "Following {agent} · {artboard}" (`follow_agent::status`) | `follow.agent`, `follow.artboard` | the newest agent edit's client; never ends |
| The AI tab's status, "Connected · Codex" (`ai_tab::connection`, `cmd::mcp::status` › `clients`) | every client that sent `initialize` in the last 10 minutes | `mcp.client` notices |
| The AI tab's Activity feed (`ai_tab::activity`) | `history.changes`, the newest 30 | the change log (history, not state) |
| The reveal (`reveal::tick`) | each landed batch | the change log; not a working state |

## Where MCP requests enter

| Route | Session id | `clientInfo` | How it reaches the window |
|---|---|---|---|
| **The app's HTTP endpoint** (`figcraft_mcp::http` → `Server::handle_session`) | Yes: `Mcp-Session-Id`, issued on `initialize` (J1) | Yes, per session (J1) | Every tool call, reads included, is a `ControlRequest` with `route: "mcp (http)"` and `client` (`apps/figcraft/src/mcp_endpoint.rs`). `initialize` sends `mcp.client` without waiting |
| **`scrim mcp --attach PORT`** (a stdio server in the CLI, `Backend::Attached`) | No (stdio) | Held by the CLI's `Server`, **never sent to the app** | Each call is a control-channel request (`route: "control channel"`, `client: None`) on one TCP connection that lasts as long as the stdio client |
| **Headless `scrim mcp`** | No | In-process only | No window: nothing to indicate |
| **The control channel itself** (scripts, tests, I4's captures) | No | No | `route: "control channel"` |

- **Ending a session:** the HTTP endpoint answers `DELETE /mcp` with 405 ("no session to end"). Streamable HTTP lets a client end its session with `DELETE` and its `Mcp-Session-Id`, so nothing ends a session today. An `--attach` client ending closes its TCP connection, and the control server's connection thread returns; nothing is told.

## Paper, for reference only

- **What I could verify:** Paper's MCP server is configured in this session, so its tool definition is readable. `finish_working_on_nodes` is described as "MUST call this when done working. Remove the working indicator from artboards you were editing." It takes a `fileId` and optional `nodeIds`, and "with no arguments" releases all of them.
- **What I couldn't verify:** how Paper decides an agent is working before it calls that, or what happens when it never calls it. No Paper code was read; nothing is copied.

## The plan

### IB: one working state, per client (`crates/ui-egui/src/working_state.rs`)
- **A client** is keyed by its HTTP session id. An `--attach` client is keyed by its control connection, and anything else by its route. It carries its display name (J1's rules).
- **Working** means, for any client:
  - a request of its is queued or waiting (a screenshot waits for a frame);
  - **or** its last request of any kind was less than the grace period ago;
  - **or** it began work and hasn't finished.
- **It ends** when:
  - the client calls `finish_working`;
  - its session ends (HTTP `DELETE`, or the `--attach` connection closing);
  - **or** the grace period runs out. Expiry is at exactly last request + grace, on the window's clock.
- **The sources:**
  - every request answered for an MCP client (HTTP; `--attach` once it names its client);
  - every agent edit that lands from any route, so the control channel and plugins keep I2's behaviour. Their reads don't count, because they aren't MCP clients.
- **`--attach` names its client:** the CLI's server sends `mcp.client` on `initialize`, as the HTTP endpoint does. The control server remembers it per connection and tags that connection's requests with it. When the connection closes, it tells the window the session ended.
- **HTTP `DELETE /mcp`** with a known `Mcp-Session-Id` ends that session (200); otherwise 404. `GET` stays 405.
- **The grace period:** `Settings.agent_grace_secs`, default 15, saved (`settings_version` 5). The command `pref.agentGrace {seconds: 5|15|30|60}` refuses other values, and has its MCP route. Settings ▸ General ▸ Agents at work: "Keep the working indicator for … after the last agent request".
- **`finish_working {ids?}`:** a new MCP tool over the command `agent.finish-working`.
  - **Access:** Read, so it runs with only read-only tools on. It's listed in Tool access and `list_commands`.
  - **Effect:** it asks the window to end the calling client's state (`Session::request_ui`, which isn't a dialog, so the control channel keeps it). It never touches the document, undo or the revision.
  - **The reply:** `{finished: true, client}`, plus `ids` echoed. With `ids`, the state still ends for the whole client (see disagreement 3).
- **The tool descriptions:** each tool that isn't read-only gets one short line: "When you have finished working on the canvas, call finish_working so the person sees you are done." The instructions string says it once too. *(Trimmed in Fix L1: now five tools carry "When done, call finish_working."; see L1 below.)*
- **Every reader reads the one state.** The dots, the outline and its pulse, the AccessKit status, `ui.inspect`, the load budget, follow's "Following {name}" and the AI tab all read it, so nothing keeps its own timer:
  - **The name** is the most recently active working client's; when none is working, the last edit's.
  - **The dots** fade in over 160 ms when work first starts, stay continuous across gaps, and fade out over 400 ms only when the state ends.
  - **The static row** (Animations Off, reduced motion, degraded) stays for the whole state too.
  - **"Show working indicator" off** still hides only the dots.
- **AccessKit:** "{name} is working on {artboard}" for the whole state. When it ends, "{name} finished" (polite) for 2 s.

### IC: reveal speed
- **`Settings.reveal_speed`:** `slow | normal | fast`, default normal (`settings_version` 5). The command `pref.revealSpeed {speed}`, with its MCP route. Settings ▸ General, next to "Reveal agent work".

| Speed | Entrance | Stagger | Batch cap |
|---|---|---|---|
| Fast (today's) | 220 ms | 40 ms | 1.2 s |
| Normal | 480 ms | 110 ms | 4 s |
| Slow | 700 ms | 180 ms | 6 s |

- **The rest stays:**
  - the stagger shrinks to fit the cap;
  - over 60 layers, top-level groups only;
  - ease-out, the 4% scale;
  - the degradation order. Step 3 caps at half the speed's cap: 0.6 s at Fast, as today.
- **A batch mid-reveal already queues behind** the running stagger (`first = last_start + STAGGER`). That stays, with the speed's stagger, so each step reads as its own reveal.
- **The camera flight** (320 ms) and the first entrance both start when the batch lands, and the flight ends while the entrance is still running at every speed. So nothing fights, and the flight is unchanged. I'll re-measure that in IE.

## Where I disagree with the brief, or read it a certain way (overrule any)

1. **The Activity feed stays history.** It lists the change log, which isn't a state. The AI tab shows the working state on its own line in the Agents section ("Codex is working on Poster"), and the feed is unchanged.
2. **Reads over the plain control channel don't keep the indicator alive.** Its scripts and tests (`ui.screenshot`, `ui.inspect`) aren't agents at work; otherwise every capture would keep the dots on. Its edits still do. `--attach`, which is an MCP client, counts fully once it names its client.
3. **`finish_working`'s `ids`:** the state is per client, not per artboard, so the call ends the client's state whatever `ids` says. `ids` are echoed in the reply. Paper's per-artboard release would need an indicator per artboard, which I2 decided against: one indicator, on the latest artboard.
4. **The grace command takes only 5, 15, 30 or 60**, the values Settings offers; anything else is refused, and nothing is ever below 5.
5. **"In flight"** is a request queued for the window or waiting for a frame. Requests are answered on the UI thread in a frame, so that window is short; the grace period carries the gaps.
6. **"{name} finished"** is spoken once, for 2 s, and only when the state ends: by `finish_working`, the session ending, or the grace running out.
7. **The tool-description line goes on every tool that isn't read-only**, including `select`, `open_file`, `undo` and the input tools. That's the plainest reading of "every mutating tool", and the cost is one short sentence. *(Overruled in Fix L1: it bloated the list every agent carries; only the five building tools keep it.)*

## IB as built

- **The state** (`crates/ui-egui/src/working_state.rs`): clients keyed by session (HTTP `Mcp-Session-Id`, or an `--attach` connection's `attach-N`), else client name, else route.
  - **One run:** it begins when the first client starts. It ends when the last one stops: at its finish, or at its last request + grace exactly, even when the frame that notices comes later.
  - **A new run during the last one's fade out** carries on from that fade's opacity, so there's no dip.
  - **`ui.inspect` › `agent`** gains `workingClients` and `finished`.
- **Where activity is noted** (`control::handle_route`, the path every agent request takes):
  - **an MCP request of any kind,** before it runs, so the client's own `finish_working` lands after it;
  - **the plain control channel** only when the request edited;
  - **a plugin's edits,** as they land.
- **The transports:**
  - `Server::with_session_hook` reports HTTP sessions starting and ending.
  - `DELETE /mcp` ends one.
  - `Backend::set_session` gives each request its known session.
  - `Server::announcing_clients` makes `scrim mcp --attach` send `mcp.client`. The control server keys that connection, tags its requests with the client and session, and sends `mcp.session-ended` when it closes.
- **`agent.finish-working`** is in the no-revision list next to Layers' collapse (`cmd::view_only`). The window ends the caller's state from the UI request.
- **Readers:**
  - **The indicator:** its alpha comes from the state. The wave runs on the clock, so it doesn't restart between steps. The artboard is let go once the run has ended and faded.
  - **AccessKit:** one live node; "{agent} finished" for 2 s where the row was.
  - **The AI tab:** a plain-text working line under the status. The indicator's node is the one live region, so it isn't announced twice.
  - **Follow's "Following {agent}":** names the working client; a nameless one falls back to J1's name for the latest edit.
  - **The load budget:** counts the state as activity.
- **Settings ▸ General ▸ Agents at work:** "Keep the working indicator for 5 s / 15 s / 30 s / 60 s after the last agent request".

## IC as built

- **`Settings.reveal_speed`** is `slow | normal | fast`, normal by default; any other value reads normal (`settings_version` 6). `pref.revealSpeed {speed}` has its MCP route. Settings ▸ General shows "Reveal speed: Slow / Normal / Fast" under "Reveal agent work".
- **`reveal::Speed`:**

  | Speed | Entrance | Stagger | Cap |
  |---|---|---|---|
  | `FAST` | 220 ms | 40 ms | 1.2 s |
  | `NORMAL` | 480 ms | 110 ms | 4 s |
  | `SLOW` | 700 ms | 180 ms | 6 s |

  `FAST` is Stage I2's constants exactly.
- **Each entrance keeps its own duration,** so changing the speed mid-reveal never warps one already running.
- **Degradation step 3 halves the speed's cap** (0.6 s at Fast, as before). The stagger shrinks first; the entrance is never shortened for load.
- **Steps queue:** a batch landing mid-reveal starts one stagger after the running batch's last entrance. None of the running entrances restart or are cut.
- **The camera flight is unchanged** (320 ms). It and the first entrance both start when a batch lands, and at every speed the entrance (220–700 ms) is still running when the flight ends. Nothing finishes "before the first reveal ticks", so there was nothing to lengthen.
- **Tests:**
  - **`reveal_tests.rs`:** Stage I2's tests now run at Fast and still pass unchanged, which proves Fast is the old behaviour. New:
    - the speed table;
    - every speed and its degraded cap, for 1, 6, 36 and 60 layers;
    - 36 layers at Normal last over 2 s and at most 4 s;
    - a step mid-reveal queues behind, at every speed;
    - render-only at Normal and Slow.
  - **`crates/mcp/tests/follow_agent.rs`:** both new settings and `finish_working` over MCP, none of it touching the document.

## ID: the tests

- **In the window, on the real request path** (`working_state_window_tests.rs`: requests on the control queue with route, client and session, as the HTTP endpoint queues them):
  - six steps of 6–36 layers 2.5 s apart keep the dots drawn in **every** frame; `finish_working` then fades them out in 400 ms ("Codex finished");
  - read-only calls (the screenshot's render, `inspect_document`, `lint_design`) keep it alive 12.5 s past the last edit with a 5 s grace period;
  - with no finish, it ends at the grace period exactly (5 s and 15 s), its fade counting from then;
  - a session closing (`mcp.session-ended`) ends it;
  - with two clients, one finishing leaves it on for the other, named after the latest;
  - an unknown name is shown as announced, and a missing one reads "An agent";
  - `finish_working` adds no revision or undo step.
- **Found by the first test:** a step landing on an artboard outside the view had no dots for the frames before the camera arrived. The row now stays on the artboard shown until the new one is in view.
- **Elsewhere:**
  - `working_state_tests.rs`: the state's own clock;
  - `agent_work_tests.rs`: the commands, Read, no revision;
  - `crates/mcp/tests/working.rs`: the tool, the hint, restricted access, sessions, `DELETE`;
  - `control_server_tests.rs`: `--attach`;
  - `reveal_tests.rs`: the speeds;
  - `crates/mcp/tests/follow_agent.rs`: the MCP routes.

## IE: verified live

The record, captures, logs and scripts are in `docs/scrim-ports/working-state/README.md`. All runs used the release build and the app's real HTTP endpoint, with an MCP client announcing Codex's name.
- **Six steps 2.5 s apart:** the dots were present in 59 of 59 captures (every 255 ms) up to `finish_working`, and gone 0.46 s after it.
- **No finish, a 5 s grace period:** the dots were present in 68 of 68 captures; the state ended at +5.0 s and the dots were gone by +5.5 s.
- **Pixels:** 0 px changed outside the canvas, dots and reveal on vs off, at seven moments.
- **Reveal durations** match the speed table: Fast 1.09 and 1.15 s, Normal 2.98 and 3.98 s, Slow 4.78 and 5.98 s, for 24 and 36 layers.
- **The 782-layer Fixture:** about 3 ms frames at Normal and Slow, and no degradation step.
- **Fixed during IE,** with a stricter window test: the row now waits for the new artboard's top-right corner to be inside the canvas before it moves.

## K2: the modal scroll leak (fixed)

- **What leaked:** while Settings (the one modal, `set_modal_layer`) was open, the wheel, trackpad scroll and a pinch over the canvas beside the dialog still panned and zoomed the camera. Follow's own check also saw the scroll as the person's, and paused following.
  - **The cause:** both read the frame's raw scroll and zoom once the pointer was over the canvas rectangle.
  - **The fix:** `dialogs::modal_input::canvas_covered` is now asked first, by the canvas's zoom and pan (one line in the frozen `canvas.rs`, not longer) and by follow's check. It says no under a modal, or under a dialog, menu or popup drawn above the canvas there.
  - **Found on the way:** the first version asked inside egui's input lock and deadlocked. It is asked after the lock now.
- **The dialogs checked** (`modal_scroll_tests.rs`, wheel, trackpad scroll and pinch over each):

  | Dialog | Leak before | Now |
  |---|---|---|
  | Settings (modal) | **yes:** the camera, and follow's pause | blocked over the whole window; works again once closed |
  | Missing fonts | no (egui's hover test) | no |
  | Local variables | no | no |
  | Export | no | no |
  | A Connect confirmation ("Run this command?") | no | no |

  The side panels' scroll areas never leaked: egui's hover test already counts a modal.
- **Tests:**
  - `with_settings_open_nothing_behind_it_scrolls_zooms_or_pauses` checks over the dialog, the canvas beside it, and both side panels. It failed before the fix.
  - `under_settings_a_scroll_on_the_canvas_doesnt_pause_following` runs on follow's harness, the only one that sizes the canvas for following. It fails with follow's check removed.
  - `over_a_dialog_the_wheel_and_pinch_dont_reach_the_canvas` covers the four floating dialogs.

## L1: the finish reminder, trimmed (Fix L)

- **What changed:** Stage I.1 put the reminder on every tool that isn't read-only (13 tools, `select`, `undo`, `redo`, `key`, `pointer`, `open_file`, `save_file`, `set_lint_profile` and `ignore_lint` among them). It is sent to every agent in every request.
  - **Now** it is on five tools only, the ones an agent builds with: `create_node`, `set_properties`, `batch`, `run_command`, `type_text` (`tools::FINISH_HINTED`).
  - **The wording**, shortened: "When done, call finish_working." `finish_working`'s own description says what it does and why, and the instructions string keeps the long sentence once.
- **The size of the whole `tools/list` response** (compact JSON as sent; tokens approximated as characters ÷ 4):

  | When | Tools | Characters | ≈ Tokens |
  |---|---|---|---|
  | Before Stage I.1 (`e46c546`) | 24 | 28,836 | 7,209 |
  | After Stage I.1 (`30eae02`) | 25 | 30,780 | 7,695 |
  | After L1 | 25 | 29,554 | 7,389 |

  L1 takes back 1,226 of the 1,944 characters Stage I.1 added. What remains is the `finish_working` tool itself and the five reminders.
- **Tests** (`crates/mcp/tests/tool_list_budget.rs`):
  - an exact allow-list: it fails if any other tool carries the reminder, or one of the five loses it;
  - a size budget: it fails if `tools/list` grows more than 2% over 29,554 characters, unless the baseline in the test is changed on purpose.
  - Each was checked by breaking it on purpose: adding `select`, dropping `type_text`, and a longer reminder each fail.

## M3: each reveal speed shows its timing (Fix M)

- **Settings ▸ General ▸ Reveal speed** lists the three speeds one a row, each with its timing:
  - "Fast (0.2 s per layer, about 1 s per step)";
  - "Normal (0.5 s per layer, about 4 s per step)";
  - "Slow (0.7 s per layer, about 6 s per step)".
- **The control:** radio rows replace the segmented control, which these labels don't fit.
- **Unchanged:** the saved values (`fast`, `normal`, `slow`), `pref.revealSpeed` and the default.
- **Translations:** the Spanish catalog has all three. The old one-word "Fast" and "Slow" keys are gone; "Normal" stays, because other labels use it.
- **The test:** `settings_tests.rs` › `each_reveal_speed_shows_its_timing_and_saves_the_same_value`.
- **The release build's window:** `docs/scrim-ports/working-gap/m3-reveal-speed-labels.png`.

## Fix Q1: the grace is 30 s by default, and the person's alone (22 Oct 2026)

**The report:** a real session (Claude Desktop through `mcp-remote`) had the dots cut at every gap between batches.
- **Settings ▸ Diagnostics** showed `begin` and `end` alternating, each end "grace period expired", most exactly 5 s after the begin.
- **The agent's pauses** were 5–15 s: it writes the next batch meanwhile.

**Q0, what the real settings held** (`~/Library/Application Support/design.scrim.app/state.json`, read only):
- **`agent_grace_secs: 5`,** written when the window last saved its preferences (21 Oct, 20:04 BST). Nothing recorded who chose it: there was no marker.
- **Not this repository's live scripts.** `ie.py`, `l2.py`, `bridge.py` and `n3.py` set preferences through the control channel (7979), on apps they started with a scratch `HOME`. The person's window runs without a control port; it was checked: it listens only on its MCP port, with the real `HOME`.
- **So it was set in that window:** by the person in Settings, or by an agent. `pref.agentGrace` was a Write command an agent could run over MCP, and a real Codex session was connected that evening. Which one can't be told.

**What changed:**
- **The choices** are 15, 30, 60 and 120 s, and **30 s is the default** (`AGENT_GRACE_SECS`, `DEFAULT_AGENT_GRACE_SECS`). Settings shows "15 s, 30 s (default), 60 s, 120 s" under "Keep the working indicator after the last agent request". There is no 5 s.
- **`agent_grace_chosen`**, saved with the setting, is set only when the person picks a value: Settings, or `pref.agentGrace` run by the person (the palette).
- **Agents are refused:** `pref.agentGrace` from an agent's route says "only the person sets how long the working indicator stays, in Settings ▸ General". Before, it was a Write command any agent with write tools could run, and its value was saved.
- **`settings_version` 7:**
  - a saved 5 or 15 without the marker becomes 30;
  - with the marker it stays;
  - a chosen 5, no longer offered, becomes the nearest choice, 15.
  - Every file saved before Fix Q has no marker, so all of them move to 30, including the person's 5.
- **5 s for tests only:** `Session::enable_test_hooks()` (no command or app path calls it) lets `pref.agentGrace` take 5 s into `agents.grace_for_tests`. That is session state, never saved. The window reads `Session::agent_grace()`, the override if there is one, else the setting.

## Fix Q2: the window notices when the grace is too short

- **When:** a client's working state ended because the grace period ran out, and **the same client** (the same session, else name) makes a request within 120 s of that end (`working_state::CAME_BACK_WITHIN`).
- **What it keeps:** a `working` event in Settings ▸ Diagnostics, in fixed words with no content: "grace too short: {client} came back after {n} s (the setting is {s} s)". `n` is the seconds since its request before.
- **What it says, once per run:** in the AI tab, under who is working, "The indicator ended before the agent's next step. A longer setting keeps it on.", with **Open the setting**.
  - The button opens Settings ▸ General and scrolls to "Keep the working indicator after the last agent request".
  - The hint then goes, and doesn't come back this run. Later returns are still noted in Diagnostics.
- **Nothing changes the setting on its own.**
- **Code:** `grace_hint.rs`; `working_state::State::came_back`.
- **Tests** (`grace_hint_tests.rs`):
  - steps 8, 12 and 20 s apart keep the dots on in every frame at 30 s;
  - at 15 s only the 20 s gap breaks, and that return is noted and hinted, once;
  - an agent's `pref.agentGrace` through the window is refused.
  - `diagnostics_tests.rs` covers the event's words and the scan for secrets.

## Fix Q3: live checks run on scratch settings

- **`SCRIM_SETTINGS_DIR`** (`settings::SETTINGS_DIR_VAR`) names the settings folder. It covers the app's preferences and recovery copies, the CLI's headless MCP and the bridge's endpoint note, and the FigCraft copy is skipped.
- **The control method `app.settings-dir`** answers `{dir, override}`.
- **Scrim's rules 9** (CLAUDE.md, AGENTS.md): every live run uses a scratch settings folder and `HOME`. `cargo xtask live-scripts` fails a script that changes a setting without naming the override.
- **The scripts already in `docs/`** (`ie.py`, `l2.py`, `bridge.py`, `n3.py`) now refuse to run unless the app says `override: true`. `p3.py` starts the app and the bridge with it.


## R3a: transitions logged as they happen, and no napping while agents work (22 Oct 2026)

**The report:** in a real session through `scrim mcp-bridge`, Settings ▸ Diagnostics showed the `begin` but no `end` after `finish_working` answered `finished: true`.

**The cause:**
- **The state only moved when the window drew.** A request marked its client as working (`touch_as`) and `finish_working` marked it finished, but the transitions were worked out and logged only in the frame's tick (`follow_agent::before_canvas`).
- **A hidden window draws no frames.** Minimised or covered, eframe runs only the app's `logic`, which still answers requests (`control::drain_hidden`). So the end was never logged until the window was drawn again, and a grace period never ran out.
- **Its clock was stuck too:** while hidden, egui's clock is the last frame's, so a request was stamped with that old time.
- **App Nap:** macOS slows a hidden app's timers, so even a wake-up asked for at the grace period's end could come late.

**What changed** (`crates/ui-egui/src/working_log.rs`):
- **Processed and logged at once:** `control::handle_route` settles the state after every agent request: the `begin`, and the `end` that `finish_working` causes, reach Diagnostics as the request is answered. Frames do the same, and log nothing twice.
- **Without frames:** the desktop app's `logic` settles the state each pass, hidden or not (`working_log::logic`, called from `apps/figcraft`). A wake-up is asked for at the next grace period's end, so a hidden window ends the state on time.
- **The clock** is egui's while the window draws. While hidden, it carries on from the last frame by the wall clock. It never goes back.
- **Every transition wakes the window** (`request_repaint`), so what it shows catches up when it is next drawn.
- **Follow still hears of the end:** the cause is kept until the next frame takes it (Fix N clears a pause then).
- **No napping while agents work:** the platform is asked to keep the app awake from the state's start to its end (`Services::keep_awake`).
  - **macOS** (`apps/figcraft/src/app_nap.rs`): an `NSProcessInfo` activity, "user initiated, allowing idle system sleep". It prevents App Nap and timer coalescing but doesn't keep the computer awake.
  - **Let go by releasing it:** `endActivity:` is unsafe to call, which the workspace forbids, and the activity ends when its token is released. Checked on this machine: the system lists the activity while it is held and drops it once released, both inside an autorelease pool.
  - **Other platforms:** nothing to hold.

**Tests:**
- `working_log_tests.rs`:
  - `finish_working` logs `end … finish_working` with no frame drawn, and wakes the window;
  - a grace period ends while the window is minimised, in `logic` alone (the clock 6 s on, past a 5 s grace), logged `end … grace period expired` once;
  - the wake-up is asked for at the grace's end;
  - a visible window reads egui's clock;
  - the app is kept awake from the first request to the end, and never while idle;
  - frames after a logged end log nothing twice, and follow still gets the end.
- `apps/figcraft/src/app_nap_tests.rs`: one activity while working; none while idle; the real activity is listed by `pmset -g assertions` while held and gone once released (with options the system lists).
- **Mutations:** without the request path's settle, three fail; without the hidden clock, the grace test fails.

**Not verified live:** a real Claude Desktop session with the window hidden. Activity Monitor's App Nap column can't be read from a script; the activity the app holds isn't one `pmset` lists.


## Fix T1: the grace is 60 s by default (22 Oct 2026)

**The evidence:** in a real Claude Desktop session through `scrim mcp-bridge`, an agent paused 36 s between two batches of 18 layers. At the 30 s default the working state ended and began again: the dots dropped for about 6 s, and Diagnostics logged "grace too short: … came back after 36 s (the setting is 30 s)". Fix Q2's self-diagnosis worked; the default didn't cover real use.

**What changed:**
- **The default is 60 s** (`DEFAULT_AGENT_GRACE_SECS`). The choices stay 15, 30, 60 and 120 s, and Settings shows "15 s, 30 s, 60 s (default), 120 s".
- **The migration** (`settings_version` 8): any saved grace the person didn't choose (no `agent_grace_chosen`) becomes 60. A chosen one stays, 30 included.
- **Agents still can't change it:** `pref.agentGrace` is the person's alone.
- **The hint's wording names no number,** so it is unchanged.

**Tests:**
- `agent_work_tests.rs`:
  - the default is 60;
  - unmarked 15, 30 and 60 become 60, and a chosen 30 stays 30;
  - agents are refused.
- `grace_hint_tests.rs`:
  - at the default, gaps of 20, 36 and 50 s keep the dots on in every frame;
  - at a chosen 30 s only the 36 s gap breaks them. "grace too short: … came back after 36 s (the setting is 30 s)" is logged, and the hint shows once.
- `settings_tests.rs`: the picker's labels.

**Decision (overrule it):** the brief says a saved value with no marker becomes 60. That includes an unmarked 15 or 120, which only an older default, a script or an agent from before Fix Q1 could have left. The person's own picks have been marked since Fix Q1.
