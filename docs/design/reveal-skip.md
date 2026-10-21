# Why 16 of 36 layers skipped their entrance at Fast (Fix L2)

**21 Oct 2026.** The observation, the hypothesis, the reproduction, and what was and wasn't found.

## The observation

In Stage I.1's first live run at Fast (`docs/scrim-ports/working-state/logs/speed-fast`), only 20 of a 36-layer step entered. The other 16 were drawn final at once. The cause wasn't found then, and that run didn't record follow's state.

## The hypothesis (from the brief): not confirmed

The hypothesis: layers created while the camera is still flying toward the follow target count as off screen, so at Fast the first layers of a build lose their entrance.

**The code says otherwise.** In `reveal::tick`, a new layer enters if it is "seen":

```rust
let seen = |id| following || view.is_some_and(|(page, r)| /* the layer meets the view */);
```

- **While follow is `following`,** every new layer counts as seen, wherever the camera is, flight or no flight.
- **The view decides only when follow is `paused` or `off`.** Then layers outside the view are drawn final, as Stage I's addendum specifies ("Layers created off screen skip the entrance and are drawn final at once").

**The reproduction agrees.** On a blank file, a step (an artboard, then 35 rectangles as one batch) landed at x 760:
- with the camera far away, at (−6000, −4000);
- or with the camera already on it.

It ran over the app's real MCP endpoint, in the release build, on a fresh app and scratch home each time. Every layer entered at Fast, Normal and Slow, through the whole camera flight, as follows.

### First build: 15 runs

| Run | Speed | Camera | Layers | Entered | Drawn final (reason) | Follow during the run | Swap-outs |
|---|---|---|---|---|---|---|---|
| fast-far-1 | Fast | far (−6000, −4000) | 36 | 36 | 0 | following | 40,458 |
| fast-far-2 | Fast | far | 36 | 36 | 0 | following | 0 |
| fast-far-3 | Fast | far | 36 | 36 | 0 | following | 9,660 |
| fast-target | Fast | on the work | 36 | 36 | 0 | following | 560 |
| fast-ie-1 | Fast | Stage I.1's sequence (24 at x 0, then 36 at x 760) | 60 | 60 | 0 | following | 0 |
| fast-ie-2 | Fast | Stage I.1's sequence | 60 | 60 | 0 | following | 0 |
| fast-ie-3 | Fast | Stage I.1's sequence | 60 | 60 | 0 | following | 0 |
| normal-far-1 | Normal | far | 36 | 36 | 0 | following | 0 |
| normal-far-2 | Normal | far | 36 | 36 | 0 | following | 24,828 |
| normal-far-3 | Normal | far | 36 | 36 | 0 | following | 15,412 |
| normal-target | Normal | on the work | 36 | 36 | 0 | following | 16,168 |
| slow-far-1 | Slow | far | 36 | 36 | 0 | following | 0 |
| slow-far-2 | Slow | far | 36 | 36 | 0 | following | 0 |
| slow-far-3 | Slow | far | 36 | 36 | 0 | following | 0 |
| slow-target | Slow | on the work | 36 | 36 | 0 | following | 49,492 |

- **The camera did fly:** each far run ended with the view on the work (cx 1080, cy 240), from (−6000, −4000).
- **Conditions:** the load average was 12–91 and several runs swapped. Their timings would be INVALID, and none are reported; the counts don't depend on timing.

### Stage I.1's own script, unchanged: 5 runs

`ie.py speed fast`, the script of the original run, with its window screenshot between the steps. All 5 runs entered 24 of 24, then 36 of 36. Follow was `following` at the end of each, with no pause recorded.

### Final build: 15 runs, with a paused case

The code committed with this record, and a third camera case: a 6-layer step in view, then a wheel turn over the canvas (the person's input, through `ui.input`), then the 36-layer step at x 20000.

| Run | Speed | Camera | Layers | Entered | Drawn final (reason) | Follow during the run | Swap-outs |
|---|---|---|---|---|---|---|---|
| fast-far-1 … 3 | Fast | far | 36 each | 36 each | 0 | following | 0 |
| fast-target | Fast | on the work | 36 | 36 | 0 | following | 0 |
| fast-paused | Fast | paused by a wheel turn | 42 | 6 | 36 (off screen, follow paused) | following → paused (the canvas input) → following (see below) | 0 |
| normal-far-1 | Normal | far | 36 | 36 | 0 | following → paused (the canvas input, see below) | 0 |
| normal-far-2, 3 | Normal | far | 36 each | 36 each | 0 | following | 0 |
| normal-target | Normal | on the work | 36 | 36 | 0 | following | 0 |
| normal-paused | Normal | paused by a wheel turn | 42 | 6 | 36 (off screen, follow paused) | following → paused | 0 |
| slow-far-1 … 3 | Slow | far | 36 each | 36 each | 0 | following | 0 |
| slow-target | Slow | on the work | 36 | 36 | 0 | following | 0 |
| slow-paused | Slow | paused by a wheel turn | 42 | 6 | 36 (off screen, follow paused) | following → paused | 0 |

**Diagnostics:** each paused run wrote two events to Settings ▸ Diagnostics:
- "following paused: the person's press, scroll or pinch on the canvas";
- "agent layers: 6 entered, 36 drawn final at once (off screen, follow paused: 36)".

## What the original run shows

**Follow was paused.** That run's last window capture (`docs/scrim-ports/reveal-skip/original-fast-36.png`) shows it:
- the paused "Follow agent" chip at the top of the canvas;
- "Paused: you moved the camera or edited" and Resume in the AI tab;
- the camera where the script's `view.set` had put it (cx 320, zoom 0.6). Step 1's left edge is at x 555 on the 905 px canvas, centred at 747: (747 − 555) / 0.6 = 320.

The good run's capture (`original-fast-b-36.png`) says "Following Codex · Step 2", with the camera on Step 2.

**The 20 that entered are exactly the layers in that view.** At cx 320 and zoom 0.6 the canvas shows page x from about −434 to 1074. Step 2 starts at x 760, and its rectangles stand in columns at x 780, 856, 932, 1008, 1084 … (60 wide, 76 apart).
- The first four columns meet the view; the fifth starts at 1084, past it.
- That is 4 of 8 in each of the four full rows (16), plus 3 of the last row's 3, plus the artboard: **20**.
- The 16 drawn final are the rest.

**So the skip was the paused rule, applied as designed.** It wasn't the flight: the camera never flew, because follow was paused before Step 2 landed.

## What paused it: not established for that run

Follow pauses for exactly three things (`follow_agent::follow`): the person's edit, a camera move that wasn't follow's or an agent's, or the person's press, scroll or pinch over the canvas. That run didn't record which; nothing did then.

**Real input reached the test windows during this reproduction**, with nothing in the scripts sending it:
- **normal-far-1 (final build):** 7.7 s after the step, follow paused for "the person's press, scroll or pinch on the canvas" (diagnostics, 16:09:01 UTC). The camera moved from (1080, 240) to (1345, 162). At zoom 0.6 that is a pan of about 159 × 47 screen pixels: a real scroll. The script sent no input in that run.
- **fast-paused (final build):** 1.5 s after the step, follow went back to `following` (about 16:08:42 UTC). Only `camera.follow-agents` resumes it, and the script never sent it then. The paused chip's Resume, or the AI tab's, runs that command: someone clicked it.

The app windows these runs open sit on the machine's real screen, and the person at the machine can scroll or click in them.
- **The most likely trigger for the original run** is the same: a real scroll, click or pinch while that run's window was in front, before the script's `view.set` put the camera back.
- **It is not proven:** that run recorded neither the pause's reason nor its time.

## Also seen: entrances cut short under swap

One intermediate run (the final build before its last change to the diagnostics event, slow-target, 12,600 swap-outs) entered 36 of 36. The diagnostics then counted 29 as "entrance cut short (dropped frames, a hidden window or a window capture)".
- **The rule behind it:** one stall over 0.25 s re-times a reveal; a second, or one over 1 s, finishes it (Stage I2's dropped-frame rule).
- **Its log** was overwritten when the set was run again, so only the count, copied from the console, is recorded here.
- **This is a second way layers can appear without their whole entrance:** a loaded machine. It isn't what happened in the original run, which had no swap-outs and showed the paused chip.

## What changed

**No fix:** the hypothesis wasn't confirmed, and the skip followed the rules as designed. The brief's fix (count the follow target as visible during the flight) is already how the code behaves. Nothing else changed in the reveal.

**The counter stays, as diagnostics** (`5d2d8bd`, the commit before this one):
- **`ui.inspect` › `agent.reveals`:**
  - `entered`: since the window opened;
  - `skipped`: by reason;
  - `recent`: the last 16 edits' tallies, each with follow's state when it landed.
- **`ui.inspect` › `agent.pausedBy`:** why following last paused, until it resumes.
- **Settings ▸ Diagnostics, two new allow-listed kinds** (`settings::diagnostics::{Pause, Skip}`):
  - `follow`: one event per pause, with its reason;
  - `reveal`: entered vs drawn final, by reason. One event per burst, once the edits have been quiet 2 s, and only when something was skipped.
  - Both carry only fixed reasons and counts: no names, ids or content. `no_secrets_reach_the_events` covers them.
- **The reasons a new layer can be drawn final:**
  - Reveal agent work off;
  - Animations Off or reduced motion;
  - landed while the window was hidden;
  - undo or redo;
  - off screen, follow paused;
  - off screen, follow off;
  - entrance cut short.
- **Tests** (`reveal_skip_tests.rs`):
  - the diagnostics, the pause reason and both events;
  - far camera, following: all 36 enter at Fast, Normal and Slow, through the flight, none cut short. It fails if "following counts as seen" is removed;
  - camera on the work: all 36 enter;
  - paused, off screen: all 36 drawn final, and final when scrolled to;
  - mid-flight and mid-reveal, the MCP screenshot (a render of the document) and an SVG export equal those after.

## Not verified

- **What paused follow in the original run:** see above.
- **The intermediate run's 29 cut-short entrances:** its log is gone. The rule is covered by `reveal_tests.rs` (`a_long_landing_frame_delays_the_reveal_and_dropped_frames_finish_it`).
- **Timings:** none are reported. The first build's runs had the load at 12–91 and several swapped.
- **A real agent client:** the MCP client was a script announcing Codex's name, as in Stage I.1.

## The files

`docs/scrim-ports/reveal-skip/`:
- `l2.py`: one run;
- `run.sh`: the final set;
- `logs/first/`, `logs/final/`: each run's samples, tallies, diagnostics events and conditions;
- `logs/original-script/`: the five `ie.py` runs;
- the two original captures, at half size.

`l2.py` imports `d3.py`, the control-channel helper the IE scripts use (not committed, as for IE).
