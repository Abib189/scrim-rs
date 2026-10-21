# Follow the agent, and reveal its work on the canvas (Stage I)

**I0, 21 Oct 2026.** What the code holds today, read from it, and the plan for I1–I4.

**The problem (a user report):** while an agent works through MCP, the canvas stays where the person left it, so the work happens off screen. The person wants:
- the camera to go to the agent's work and stay with it;
- a quiet, staggered reveal as layers arrive, instead of everything popping in at once.

## What exists

### Which edits are an agent's
- **The author of every command** is set by its route (`crates/engine/src/agents.rs`):
  - `Author::Person` for the app's own input;
  - `Author::Agent(route)` for everything else: `"mcp (http)"` (the app's MCP endpoint), `"control channel"`, `"mcp"` and `"cli"` (headless), and `"plugin …"`.
  - A caller never names itself: `control::handle` sets it per request, and the HTTP endpoint goes through the same queue.
- **Three things record it:**
  - **The undo label:** `Agents::step_label` prefixes "Agent: ". Only undoable commands have one, and a live session's steps live elsewhere.
  - **The change log** (`Session::note_edit`, one per document `uid`, the last 2,000): `Change {revision, author, command, label, ids}`. `ids` are the layers added, changed or removed (`changed_ids`, by `Arc` identity). Canvas gestures are logged too (`note_gesture`), as the person's.
  - **`history.changes {since, last}`**, the MCP tool `get_changes`: that log as JSON (`agent: true|false`).
- **The single source of truth is the change log's `author`.**
  - It is written once, after the command succeeded and the document changed, for every route, gestures included.
  - Undo labels miss non-undoable edits and live sessions.
  - The revision moves for view and selection changes too, so it says nothing about who did what.
  - Follow and reveal read only `Change`s whose `author.is_agent()`. A person's edit can't trigger either; a test pins it.
- **Edits that never reveal:**
  - **Undo and redo** are logged with `command` `edit.undo` / `edit.redo` (an agent's undo too).
  - **Opening a file** gives a new document `uid`, with an empty log.

### The "No agent" chip, the AI tab, and following
- **There is no "No agent" chip** in the top bar, or anywhere: no such string or widget exists. The top bar has the tabs, the theme toggle and the live session's people. What says whether an agent is there is the AI tab's status row (`ai_tab::connection`):
  - **Connected · {client name}:** an MCP `initialize` in the last 10 minutes (`Agents::mcp.clients`, reported by `mcp.client`);
  - **Not connected;**
  - **Server stopped.**
- **The AI tab's Activity feed** (`ai_tab::activity`) lists the newest 30 log entries:
  - "Agent (route): label" in the accent, "You: label" in the text colour;
  - a row click runs `ai.selectChange`, which **only selects** (no camera move today);
  - an agent's latest change has Undo (`ai.undoChange`).
- **There is no follow mode** and no "Follow agents as they work" control.

### The camera (D2)
- **`camera.focus-layer {id | ids}`** (`crates/engine/src/cmd/camera.rs`):
  - `reveal()` makes the smallest move with a 32 px margin and **the zoom kept**. A layer bigger than the view gets its leading edge at the margin; **nothing ever zooms**.
  - Another page is shown first.
  - It sets the camera at once and asks the window to fly (`camera.fly`).
- **The window** (`crates/ui-egui/src/camera_flight.rs`):
  - a 320 ms ease-in-out flight, the zoom eased geometrically;
  - a jump with Animations off, reduced motion or a page switch;
  - **cancel on input:** each frame compares the view with what the flight last wrote, so any other move stops it;
  - a focus asked for before the canvas has its real size is kept and applied after the first sizing.
- **Follow reuses all of this.** It runs `camera.focus-layer` (its function, not `execute`, so it adds no revision or journal entry), then the same `camera.fly` flight.
  - **One addition:** a `fit` parameter. When the work doesn't fit at the current zoom, it zooms out just enough to fit it with the 32 px margin. Without `fit`, D2's behaviour is unchanged.
  - No second animator is written.

### How the canvas is drawn
- **Each frame** `canvas::request_render` builds a `render_worker::Job`: an `Arc<Document>`, the page and the view.
- **A worker thread** renders it on the CPU (tiny-skia). `figcraft_render::Retained` keeps the last image and re-renders only the areas of nodes whose `Arc` changed.
- **The UI shows the newest finished image,** re-projected to the current view.
- **The job already takes a render-only document:** `app.preedit_view()`, the live text edit, is a derived `Arc<Document>` drawn in place of the session's, never written back.
- **That is where the reveal goes.** While a reveal runs, the job gets a derived copy of the document: the same nodes, with the revealing layers' `opacity` and `transform` (a 4% scale about their centre) eased. `imbl` shares every other node.
  - Nothing else reads this copy. The session's document, hit-testing, selection, undo, the change log, `inspect_document`, exports, saves and MCP results never see it.
  - `Retained` re-renders only the revealing layers' areas each frame.
- **Overlays** (selection, handles, the agent outline) are egui shapes in screen space, from the session's document (`FigApp::to_screen`).

### What `screenshot` and export render from
- **MCP `screenshot`** (no `window`): `Backend::render_png` → the control method `ui.render` → `file.export {format: png}`. That renders the **session's** document through `render_node`, never the canvas image, so the reveal can't reach it. Exports, `inspect_document` and saved files are the same.
- **MCP `screenshot {window: true}`** and the control channel's `ui.screenshot` capture the window's own pixels, which is what the person sees. A window capture asked for mid-reveal ends the reveal first (I2). It waits until the canvas shows the final document, so it shows the final state.

## Plan

**I1, follow** (`crates/ui-egui/src/follow_agent.rs`; `cmd/camera.rs`):
- **The command:** `camera.follow-agents {on?}` → `{on, state: following | paused | off}`, with an MCP route.
  - With `on: true` while paused, it resumes and flies to the latest work. Without `on`, it toggles.
  - Its label is in the Actions palette.
- **The setting:** `Settings.follow_agents` (default on), and `SETTINGS_VERSION` 2 → 3.
  - Toggles in the AI tab ("Follow agent") and in Settings ▸ General.
  - The paused state is the window's own, mirrored in `Agents::follow_paused` so the command can answer it. It is never saved.
- **What triggers a move:** each frame, the log's agent changes since the last one seen.
  - **The target** is the union of their layers' bounds on the page of the newest one; a layer that no longer exists is left out.
  - **Whole frames:** a new top-level frame is framed whole. More than 60 layers in a burst frames their top-level frames, not every leaf.
- **The move:** `camera.focus-layer {ids, fit: true}`, then the D2 flight.
- **Debounce:** the first move goes at once. While edits keep coming, at most one new flight per 400 ms, each to the latest union. A new flight starts from where the camera is, so it never jumps back.
- **The person wins.** Follow pauses (it doesn't turn off) on:
  - a press on the canvas;
  - a scroll or pinch over it;
  - the person's own edit (any key or panel change that edits);
  - any camera move that isn't follow's or an agent's own (a zoom key, a Layers click).
- **While paused:** a "Follow agent" chip at the top of the canvas and in the AI tab. One click (or the command) resumes and flies to the latest work; nothing else resumes it.
- **Mid-gesture:** with the pointer down anywhere, a text edit open, or a canvas interaction in progress, the move waits and goes after.
- **Animations off or reduced motion:** a jump cut, the flight's existing rule.
- **The AI tab:** "Following Claude · {artboard}". The name is the MCP client's ("Claude" for Claude's clients); the artboard is the top-level frame of the latest work. An Activity row now selects **and focuses** its layers (D2's focus).

**I2, the reveal** (`crates/ui-egui/src/reveal.rs`):
- **Layers an agent creates:**
  - 220 ms each: opacity 0 → 1 and scale 96% → 100%, ease-out cubic;
  - staggered in creation order (ascending ids), 40 ms apart;
  - the stagger shrinks so a batch never takes more than 1.2 s;
  - over 60 layers, only the roots of what was created (their top-level groups) are revealed.
- **An accent outline** (`ink::overlay()`, G1) is drawn around the agent's newest or last edited layer. It fades over 600 ms once the agent has been idle for a moment.
- **A move** by an agent, where only the layer's position changed, tweens 160 ms. Other property edits are skipped as not cheap and safe.
- **Deleted layers** fade out over 160 ms, only when the change only deleted (the previous document then draws for those 160 ms, the deleted layers fading). Otherwise they vanish at once.
- **Skipped for:** the person's edits; undo and redo; file open; headless runs (no window); Animations off or reduced motion.
- **Finished at once** when the window draws no frames (a gap over 250 ms), when a render goes over budget, and on a window capture.
- **Cost:** with no reveal running, the per-frame check allocates nothing (a length compare on the log). While one runs, the derived document clones only the revealing nodes. Frame times on the 782-layer Fixture are reported in I4.

## Where I disagree with the brief, or read it a certain way (overrule any)

1. **The "No agent" chip doesn't exist.** I don't invent one. The paused "Follow agent" chip is the only new top-bar-area chip; the agent's presence stays in the AI tab's status row.
2. **"Renderer-only" means a render-only document handed to the canvas job,** not new state inside `figcraft-render`. The renderer stays a pure function of what it's given, and the derived document is the same channel the live text edit already uses. The session's document is never touched.
3. **Follow zooms out only to fit, and never zooms in.** "Keeping the zoom unless the work does not fit."
4. **An agent's own camera moves** (`view.set`, `camera.focus-layer` over MCP) don't pause follow. Only the person's do.
5. **Any edit by the person pauses follow,** not only a key: a panel edit means they are working too.
6. **Property tweens are limited to moves,** and the delete fade to delete-only changes. Fills, sizes and text change at once, as the brief allows ("only where cheap and safe; if not, skip them").
7. **"Claude" in "Following Claude"** comes from the MCP client's name: any client named like Claude reads "Claude", another client its own name, and the control channel or a plugin "an agent". It is never hard-coded.
8. **The 400 ms limit** applies between flights; the first flight of a burst goes at once.

## Addendum (21 Oct 2026): the working indicator, and the reveal beside it

Two amendments to the brief (read before I2). The reveal still plays, and a **working indicator** (dots and a handle-free outline) is shown **in addition** to it, never instead of it.

### Decided from the code
- **The group gaps are a fixed pattern.** The code has no indicator, so nothing constrains this; it is decided here. The 20 columns are four groups of five, with a fixed extra gap between groups, and the wave runs through them. This is why:
  - The width stays constant, so E2's label layout can place the row once, without re-laying-out every frame.
  - Fixed geometry is testable.
  - A gap that moved would read as the row breaking apart.

  If the screen recording shows the gaps moving, it is a change to the wave function only.
- **Its size is in screen points,** zoom-independent like the labels. The screenshot reads as a 2× capture, so its "8–10 px dots" become 4 pt dots with a 3 pt gap, and 3 pt between groups. That makes the row 149 × 11 pt, 12 pt above the artboard's top edge, right-aligned to it.
- **It joins E2's layout step** (`canvas_labels.rs`):
  - The titles, status icons and section labels are placed first, as they are today; they win.
  - The indicator then takes the right end of its artboard's row, if that stretch is free with E2's gap either side. Otherwise it drops columns from the left, one at a time, down to five (one group). With less room than that it hides.
  - It is never wider than its artboard on screen, so it never sits detached from it.
- **Owner:** the indicator belongs to the latest top-level artboard the agent worked on in a burst, never to leaf layers.
- **The outline:** that artboard gets a 2 px solid outline in the canvas overlay colour (`ink::overlay()`, G1). It has no handles and no hit area; it is only painted. If the person has that artboard selected, the selection's own outline and handles are drawn and this one is not, so the person's wins.
- **The wave:**
  - **Opacity and radius:** each dot's are `0.5 − 0.5·cos(2π·(t/1.6 + phase(row, column)))`, where the phase is a hash of the column seeded per artboard, plus a small offset per row.
  - **Timing:** it loops every 1.6 s, cosine-smooth, so nothing pops.
  - **Fades:** in over 160 ms when work starts; out over 400 ms once the agent has been idle for 600 ms.
  - **Determinism:** it is a pure function of (seed, column, row, time), and tests pin it.
- **Static row:** with reduced motion, Animations Off, or "Show working indicator" degraded, the row is drawn at a fixed point of the wave, with no time.
- **Accessibility:** one AccessKit node, a polite live status: "Claude is working on {artboard}" (or "{agent} is working on …"). The agent's name and the artboard are the same ones the AI tab's "Following …" text uses. There is no node per dot.
- **Contrast:** the dots are decorative. The row's brightest dot (full opacity, `ink::overlay()`) reaches 3:1 against the page behind it in both themes; G1's overlay rule already guarantees that, and a test asserts it.

### The reveal beside it
- **Both run together:** the reveal and the indicator never suppress, delay or replace each other. A test runs a 16-layer batch and asserts both are active in the same frames.
- **Follow paused:** new layers still reveal if they are in the viewport when they arrive.
  - While following, the work the camera is flying to counts as in view.
  - Layers created off screen skip the entrance and are drawn final at once, never left hidden or half-faded.
- **Settings ▸ General ▸ Animations** (Follow system / On / Off) governs both. With Off, or reduced motion, layers appear final and the indicator is the static row.
- **Two more switches, both on by default and saved:**
  - **"Reveal agent work":** `pref.revealAgentWork {on?}`;
  - **"Show working indicator":** `pref.workingIndicator {on?}`.
  - Both are commands, so they have MCP routes. `settings_version` 3 covers them with `follow_agents`.
- **Degrading under load** (replaces I2's cost rule). Steps are taken in this order, and the next only while frames are still over budget:
  1. the dot wave becomes the static row;
  2. the outline's gentle pulse (its opacity breathing with the wave) stops;
  3. the stagger shortens, the whole reveal capped at 0.6 s.

  The layer fade itself is dropped only:
  - when the window is hidden;
  - when frames are skipped badly (a gap over 250 ms);
  - when the setting is off;
  - with reduced motion or Animations Off.

  I4 reports which step the 782-layer file reached.
- **Never in the document:** the reveal, the indicator and the outline are painted only. A test takes `screenshot`, an export and `inspect_document` mid-reveal and after, and asserts they are identical.

## I1 as built

- **The engine:**
  - **`camera.focus-layer {fit}`:** `reveal_or_fit` is D2's least move when the layers fit at the current zoom; otherwise it is the zoom that fits them with the 32 px margin (lower, never higher), centred on them.
  - **`camera.follow-agents {on?}`** answers `{on, state}`, with `follow_state` giving following, paused or off. Access is Write. Every locale names it.
  - **The setting:** `Settings.follow_agents` (default on), `SETTINGS_VERSION` 3. A file from version 2 reads on.
  - **Paused:** `Agents::follow_paused`, the window's, never saved.
  - **`Agents::edits_after` / `last_edit`:** reading the log costs one comparison when nothing is new.
- **The window** (`crates/ui-egui/src/follow_agent.rs`; hooks in `lib.rs` before and after the canvas, 1722 → 1728 lines of 1730):
  - **Each frame**, the new log entries are classified created, changed or deleted against the document last seen. Another file's history is never new work.
  - **The target:** the agent's layers on the newest one's page. A created top-level frame is framed whole; more than 60 layers frame their top-level frames; a layer listed only as another's parent is left out.
  - **The move:** `camera.focus-layer {ids, fit}`'s function runs directly (no revision or journal entry), then D2's flight. The first flight goes at once, then at most one every 400 ms to the gathered union.
  - **Pauses** on:
    - the person's edit;
    - a press, wheel or pinch on the canvas. The bottom 72 px, where the toolbar and help button float, and the chip don't count;
    - a camera move that isn't follow's or an agent's own.
  - **Agents' own moves:** `control::handle_route` notes an agent's own camera move (`agent_moved_camera`).
  - **Waits** while the pointer is down, a text edit or canvas interaction is open, or someone else's flight runs. Nothing moves before the canvas has its real size.
  - **Resume:** the "Follow agent" chip at the top of the canvas, the AI tab's button, or the command. The window sees the change to following and flies to the latest work.
- **The AI tab:** one row under the Claude Code status: the "Follow agent" switch, Resume while paused, and the status ("Following Claude · Poster"). The name is the MCP client's ("Claude" for Claude's); other routes read "An agent".
  - **When it shows:** once there is an agent to follow (an MCP client connected, or an agent's edit seen), and whenever following is paused or off.
  - **Why:** a row for nothing pushed the tab's other sections down for no reason; Settings and the palette always have the switch.
- **An Activity row** now selects and focuses (`camera_flight::focus_selection`).
- **Nothing to pause before an agent works:** the person's own edits, clicks and scrolls pause following only once an agent's work has been seen in the file. Otherwise every edit would show the paused chip with no agent anywhere. (Overrule if you want the brief's literal rule.)
- **Settings ▸ General ▸ Agents at work:** "Follow agents as they work".
- **Tests:** 14 window tests, plus two in `cmd/camera_tests.rs`. Breaking each of these in turn fails a test:
  - the person-edit pause;
  - the 400 ms limit;
  - the mid-gesture wait;
  - the size wait;
  - the agent-camera hook.

## I2 as built: the reveal and the working indicator

- **Settings:** "Reveal agent work" (`pref.revealAgentWork`) and "Show working indicator" (`pref.workingIndicator`) in Settings ▸ General ▸ Agents at work. Both are on by default and saved, with `settings_version` 4; a version-3 file reads them on.
- **The reveal** (`crates/ui-egui/src/reveal.rs`):
  - **The entrance:** 220 ms, opacity 0 → 1 and scale 96% → 100%, ease-out cubic.
  - **The stagger:** 40 ms apart in id order, the batch capped at 1.2 s (0.6 s at degradation step 3). Over 60 layers, only the roots of what was created enter.
  - **Moves and deletes:** a move that changed only the position tweens over 160 ms. A delete-only edit fades the deleted layers over 160 ms, drawn from the document before the delete.
  - **The render-only document:** the canvas job's document (`canvas.rs`, the one line that already chose the live text edit's document; still 1509 lines). It is a copy with the revealing layers eased.
  - **Classifying:** the scanner counts a layer as created only in the first edit of a frame that lists it, so a frame built child by child enters once, first.
- **Where it plays:**
  - while following, everywhere (the camera is on its way);
  - while paused or off, only for layers in view when they land. Others are drawn final at once and stay final when scrolled to.
- **The working indicator** (`crates/ui-egui/src/working_indicator.rs`), painted in E2's label step (`canvas_labels::draw`) from the labels' placed rectangles:
  - **The row:** 2 × 20 dots in groups of five; 149 × 11 pt; 12 pt above the artboard and right-aligned to it.
  - **Placement:**
    1. columns are dropped from the left where a label is in the way;
    2. when not even five columns fit there, it goes one row higher, 27 pt above the artboard, clear of the label row;
    3. failing that, it hides.

    That second place is an addition: without it, a long title hid the row at most zooms.
  - **The wave:** `wave(seed, row, col, t)`, a cosine with a 1.6 s loop and a phase per column (splitmix64 of the artboard id and the column). The fades follow `alpha`. The static row is `still` (the wave at a fixed moment).
  - **The outline:** `ink::overlay()`, 2 px. It is skipped when the artboard is in the selection, including when the agent's own new frame is selected.
  - **AccessKit:** one `Status` node with `Live::Polite`: "Claude is working on Poster".
- **Under load** (`follow_agent::Budget`): over 16.7 ms a frame, or 33 ms a canvas render, for three frames in a row while something animates, steps are taken one at a time, at least 250 ms apart:
  1. the static row;
  2. no outline pulse;
  3. the 0.6 s cap.

  It returns to 0 after 2 s idle. The fade is never dropped for load.
- **`ui.inspect`:** `agent: {follow, revealing, indicator, outline, working, degraded}`.
- **A correction to I0:** I0 said a window capture would end a reveal first. It doesn't: I4 has to capture the reveal in the real window, and a window capture is "what the person sees". MCP `screenshot` without `window` (the render), exports and `inspect_document` are what never see it, and a test pins that.
- **Tests:** `reveal_tests.rs` (9) and `working_indicator_tests.rs` (11). Breaking each of these in turn fails a test:
  - skipping undo;
  - skipping the person's edits;
  - the off-screen rule;
  - a time-independent wave;
  - the selection rule;
  - the label clearance.

## I3: the tests

Most of I3's list landed with the code it covers, in I1 and I2, as the repo's rule asks; `docs/checklists/follow-agent.md` maps each item to its test. I3 adds two:
- **The MCP route:** `run_command` answers each switch's state, `list_commands` lists them, and none of them touches the document (`crates/mcp/tests/follow_agent.rs`).
- **The system's reduced motion** (Animations: Follow system): a jump cut, no entrance, and the static row, as with Animations Off.

Over I1–I3 that is 38 tests in the window (`follow_agent_tests.rs` 15, `reveal_tests.rs` 9, `working_indicator_tests.rs` 11), plus two engine tests and one MCP test. Breaking each guarantee in turn (11 in all) fails at least one of them.

## A follow-up from I4's first run

- **Work entirely out of view is centred.** With `fit`, D2's least move parked a poster arriving from far away on the canvas's far edge, partly under the floating toolbar. The brief's least move is for work already visible. Now:
  - partly in view: the least move;
  - out of view: centred on it;
  - too big: fitted and centred.

  D2's Layers click (no `fit`) is unchanged.
- **The AI tab's status has its own line** again, under the switch. Beside the switch it was cut down to "…". The row shows only when an agent is around, so the tab's other sections keep their place otherwise.
- **A long frame no longer drops the reveal.** On the 782-layer Fixture, an agent's 15-command batch takes one long frame to run (86–1028 ms over six runs; it is the engine's batch on a big file, not the reveal). The reveal used to treat that gap as frames being dropped and finished at once, so it never played there.
  - **The rule now** (`24b163d`, then this commit):
    - The frame right after a landing is the landing's own: the reveal and the indicator are re-timed by its gap (up to 5 s), so they play after it.
    - Later in a reveal, one stall over 250 ms re-times it too; a second, or one over 1 s, finishes it (frames are being dropped).
    - Edits answered while the window is hidden (`control::drain_hidden`, which runs only then) are drawn final.
  - **Tests:** `a_long_landing_frame_delays_the_reveal_and_dropped_frames_finish_it`, `a_long_frame_after_the_agents_edit_isnt_idle_time`.
  - **Overrule** if you'd rather any long frame finish it.
- **An idle window isn't a hidden one.** The I4 runs on the Fixture still showed no reveal whenever the window had been idle before the agent's edit. egui draws no frames while nothing asks for one, so the first frame after a quiet spell has a large gap, and the rule above read that gap as a hidden window.
  - **The real signal now:** requests answered by `control::drain_hidden` (which runs only while the window draws no frames) mark what they land as seen while hidden. The gap before a landing isn't used.
  - **Test:** `an_edit_after_a_quiet_minute_still_enters` fails under the old rule.
- **One slow render isn't a slow frame rate.** On a Fixture run, a 220 ms render (the first after the window came back) stayed in `render_ms` until the next render. Every frame after it counted as over budget, and the degradation stepped all the way to 3. Only a new render counts now (`one_slow_render_doesnt_degrade`).
