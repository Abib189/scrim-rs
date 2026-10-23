# Idle repaints in the real app (Fix AA5, 23 Oct 2026)

**The question:** does the window keep repainting while nothing happens? Fix Z5 measured 0.1 a second in the test harness, which has no MCP server or file watcher, and changed nothing. Your real window (the Fix X+Y build) disagreed: about 23 frames a second over 16 minutes.

**The answer:**
- **The cause found:** a focused text field. egui's blinking caret drew 7.2 to 7.4 frames a second in the real app, for a caret that changes twice a second. The composer is focused after ⌘↩ sends, so it stayed that way after every run.
- **Now:** 2.0 a second with the composer focused (the caret's own blink), and 0 in every other case.

## How it was measured

- **The rate:** `ui.inspect` › `repaintsPerSecond` and `repaintCausesRecent` (`frame_stats.rs`). That is the frames of the 5 s before the frame serving the request, and for each call site, how many of those frames it asked in.
  - egui keeps every request of a frame as a cause, a timer due in half an hour included. So `lib.rs:1279`, the autosave timer re-asking each frame, shows in every frame without drawing any. The rate is the number to trust.
- **The script:** `docs/design/idle-repaint/aa5_measure.py`. It runs the app from this tree (`target/debug/figcraft`) on:
  - a scratch HOME and settings folder;
  - MCP on 7690 and control on 7979, never your Scrim on 7680;
  - a stub `claude` first on its PATH (`crates/mcp/tests/stub/stub_agent.py`), allowed in the scratch settings. The MCP server runs, as in the real app.
- **Each case:** the window is brought forward, the mouse moved away (except in (d) and (e)), 4 s to settle, the stats reset, then 10 s with nothing sent to the app, then one `ui.inspect`. The 3 frames counted in each 10 s are the measurement's own: the reset's two and the inspect's.
- **Case (b):** a real run, typed into the composer with `ui.input` and sent with ⌘↩: the stub made a rectangle and ended, and the card read "Done".
- **Real input reaches these windows:** in one discarded attempt your mouse passing over the scratch window drew 10 frames a second, with the cause `context.rs:538` (input). Such frames aren't idle.

## The numbers (the real app, 10 s each)

| Case | Before (the fix's parent) | After |
|---|---|---|
| (a) AI tab, empty transcript | 0.0 /s | 0.0 /s |
| (c) Design tab | 0.0 /s | 0.0 /s |
| (b) AI tab after a stub run, the composer focused (as after ⌘↩) | **7.2 /s**, all `egui text_selection/visuals.rs:313` (the caret) | **2.0 /s**, `caret.rs` (the blink, twice a second) |
| (b) the same, the composer not focused | (not measured before) | 0.0 /s |
| (d) the pointer resting on a card button | **7.4 /s**, the caret (the composer was still focused) | 0.0 /s, the composer not focused |
| (e) the mouse over the panel | **7.4 /s**, the caret | 0.0 /s, the composer not focused |

**Not verified:** that the tooltip was showing during (d). `ui.inspect` doesn't report tooltips; the pointer rested on Select layers for 1.5 s before the 10 s.

## The fix

`caret.rs`, called from `theme::apply_with` each frame:
- **What it does:** turns egui's blink off and blinks the caret itself, half a second on and half off. It hides the caret by making its stroke transparent in the off half, and restarts the blink at each key, click or focus change. It asks for a frame just after each change: 25 ms late, which is more than the frame egui takes off a delay.
- **With reduced motion** the caret is steady and costs nothing.
- **Why egui's drew so many:** it asked to be woken when the caret next changed, but egui takes a frame's time off every delay. So it woke a frame early, found the caret unchanged, and asked again with almost no delay. An immediate request draws two frames.

**Tests** (`ai_idle_tests.rs`, a fake clock that emulates the event loop):
- `a_focused_composer_draws_at_most_two_frames_a_second`: 20 frames in 10 s now, 58 on the old code.
- `repaints_per_second_stay_at_most_two_once_settled`: the guard. `ui.inspect`'s `repaintsPerSecond` after settling is at most 2 in (a), (b) (not focused, and focused) and (c). It fails on the old code in the focused case.
- `idle_repaints_stay_under_two_a_second` (Z5's, kept). Its frame counts now leave out the frame that starts each measurement.

## The suspects in your counts

Your counts (`tooltip.rs:262` 94,799; `working_log.rs:82` 17,084; `chat_ui.rs:374` 8,786; `ai_run_card.rs:75` and `:270` 4,393 each; 22,733 frames) are totals since the app started, so they include the runs and the scrolling. In the measurements above none of them asks for a repaint once a run has ended:
- **`chat_ui.rs:374` (now 378):** polls the agent's output while a run goes, and only then.
- **`ai_run_card.rs:75` and `:270` (now 78 and 398):** the banner's dots (every 100 ms) and the card's bar (every 50 ms), only while a run goes.
- **`working_log.rs:82`:** the timer for a working state's grace period ending: once, when it is due.
- **`tooltip.rs:262` (egui):** for 0.4 s after any scroll, every widget with a tooltip under the pointer asks to be woken when tooltips may show again. That is scrolling time, with the canvas scrolled by trackpad. With the mouse away it asked nothing in (a) to (e).

The average of 23 a second over 16 minutes fits runs and scrolling plus the focused composer's 7 a second between them; the totals can't say how much of each. With the fix, the time between runs costs nothing, or 2 a second while the composer is focused.

## Still above 2 a second

Nothing, once settled. While a run goes, the banner's dots (10 frames a second; none with reduced motion) and the card's bar (20 a second) animate on purpose. They stop when the run ends.
