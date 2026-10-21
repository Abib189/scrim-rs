# Stage I.1 verification (IE, 21 Oct 2026): the working state and the reveal's speed

**The setup:**
- **The app:** the release build, the build committed with this record, on a fresh scratch `HOME` per run. Window captures with `ui.screenshot` (the window's own pixels, 2880 × 1800 at 2×).
- **The agent:** an MCP client on the app's **real HTTP endpoint** (127.0.0.1:7681), announcing Codex's `clientInfo` name (`codex-mcp-client`), with its own `Mcp-Session-Id`. Every edit and `finish_working` went over it.
- **The control channel** only set the starting state, captured the window and read `ui.inspect`. Its reads don't count as an agent at work.
- **The scripts and every run's log are in `logs/`:**
  - `ie.py` drives a run;
  - `iesheet.py` makes the contact sheets;
  - `ie-report.py` prints every number below from the logs.
- **Conditions:** the machine had 16 GB of swap allocated by other apps throughout. Each run records its load average and the swap-outs during it (`logs/*/conditions.json`). Runs that swapped are marked below; their timings are INVALID by this repo's rule. The behaviour checks don't depend on timing.

## Six steps 2.5 s apart, then `finish_working` (`sheet-1-six-steps-then-finish.png`)

The build: six batches of 6, 12, 18, 24, 30 and 36 layers, each an artboard and its layers, 760 px apart, 2.5 s between steps, on a blank file. Then `finish_working`. A capture thread took a screenshot and `ui.inspect` every 255 ms (73 captures).

| Check | Result |
|---|---|
| **Dots in every frame** from the first step (+0.4 s for the fade in) to `finish_working` | **59 of 59 captures:** the row was drawn and inside the canvas (`ui.inspect`), and the sheet shows it in each tile |
| **Across the step landings** (+2.71, +5.25, +7.80, +10.35, +12.89 s) | The dots stay on the previous artboard while the new one comes into view, then move to it (`02-step-3-landing-…png`). No frame without them |
| **What it says** | AccessKit and the AI tab: "Codex is working on Step 1" … "Codex is working on Step 6"; follow: "Following Codex · Step N" (`01-mid-gap-after-step-3.png`) |
| **`finish_working`** | Answered `{finished: true, client: "codex-mcp-client"}`. 0.20 s later the row is fading (`03-…`); at 0.46 s, the first capture after the 400 ms fade, it is gone (`04-…`). "Codex finished" for 2 s, then nothing |

Conditions: load 5.4, 304 pages swapped out during the 22 s run.

## No `finish_working`, the grace period at 5 s (`sheet-2-grace-5s-no-finish.png`)

The same six steps, then nothing.

| Seconds after the last request | Dots | Status |
|---|---|---|
| +4.22, +4.47, +4.73 | drawn | "Codex is working on Step 6" |
| +4.98, +5.24 | fading | "Codex finished": it ended at the grace period's end |
| +5.49 and later | gone | "Codex finished" for 2 s |

From the first step to the last request + 4.9 s: **68 of 68** captures had the dots. No swapping during the run.

## Pixels: 0 px change outside the canvas (`05-…`, `06-…`)

- **The runs:** the six steps were run twice on fresh apps. Once with the dots and the reveal on, once with both off ("Show working indicator" and "Reveal agent work"). Both runs captured the same seven moments: 1 s after each step, and 1 s after `finish_working`.
- **The comparison:** everything outside the canvas rectangle, at full resolution (`magick compare -metric AE` with the canvas blanked).

| Moment | Outside the canvas | Inside it (the dots, the outline and the reveal) |
|---|---|---|
| after step 1 … step 6 | **0 px** each | 2,122 → 527,192 px |
| after `finish_working` | **0 px** | 171,433 px: step 6's 36 layers were still entering (Normal takes up to 4 s) |

So the working state changes nothing outside the canvas: not the tab bar, the panels, the layers list or the toolbar. The AI tab's working line and follow's status read the working state, which is on in both runs; the switches only hide the dots and the reveal. Conditions: the ON run swapped out 2,188 pages, the OFF run none.

## The reveal at Fast, Normal and Slow

One 24-layer and one 36-layer batch, in view, at each speed; `ui.inspect` sampled every ~30–50 ms; the reveal's duration is from the first to the last sample with layers entering.

| Speed | 24 layers (expected) | 36 layers (expected) |
|---|---|---|
| **Fast** | 1.09 s (1.14) | 1.15 s (1.2, the cap) |
| **Normal** | 2.98 s (3.01) | 3.98 s (4.0, the cap) |
| **Slow** | 4.78 s (4.84) | 5.98 s (6.0, the cap) |

- **The measurements** are within the sampling interval of the table.
- **The Fast row** is the second Fast run (`logs/speed-fast-b`), which records follow's state ("following" throughout). In the first Fast run (`logs/speed-fast`), only 20 of the 36 layers entered (0.92 s). The other 16 were drawn final at once, as layers outside the view are when follow isn't on its way to them. That run didn't record follow's state, so why it wasn't following then is **not established**.
- **None of the speed runs swapped.**

**The camera flight** (320 ms) is unchanged. It and the first entrance start together when the batch lands, and at every speed the entrance (220–700 ms) is still running when the flight ends.

## Frame times on the 782-layer Fixture

A 16-layer batch into "Poster — The Moon Is Late", after 2 s idle; `ui.inspect` every ~30 ms for 7.5 s.

| Run | Reveal | Frame median / max | Render median / max | Dots | Degradation step | Swap-outs |
|---|---|---|---|---|---|---|
| Normal 1 | 2.04 s | 3.5 / 85 ms | 3.1 / 150 ms | 151 of 152 samples | 0 | 0 |
| Normal 2 | 2.04 s | 3.5 / 80 ms | 3.1 / 19 ms | 150 / 151 | 0 | 0 |
| Normal 3 | 2.06 s | 3.5 / 72 ms | 2.6 / 22 ms | 148 / 149 | 0 | 0 |
| Slow 1 | 3.31 s | 2.9 / 87 ms | 3.3 / 32 ms | 149 / 150 | 0 | 0 |
| Slow 2 | 3.33 s | 2.9 / 78 ms | 3.1 / 20 ms | 150 / 151 | 0 | 0 |
| Slow 3 | 3.29 s | 2.9 / 88 ms | 3.2 / 49 ms | 150 / 151 | 0 | 0 |

**How to read it:**
- **The reveal's length:** 16 layers enter in 2.04 s at Normal (2.13 expected) and 3.3 s at Slow (3.4 expected).
- **Frames and renders:** frames stay around 3 ms and renders around 3 ms while it runs. No degradation step was reached.
- **The maximum frame** (72–88 ms) is the frame the agent's batch lands on, on this big file. I4 measured 150–170 ms for its batch. The render maximum of 150 ms in Normal 1 is one render.
- **The sample without dots** in each run is the first, taken before the batch landed.
- **Swapping:** none of these runs swapped. The 16 GB of swap was allocated throughout, though, so treat absolute times with care.

## Found and fixed during IE

- **A step landing beside the view:** its artboard overlapped the canvas, but its top-right corner, where the row hangs, was off screen. The row moved there, so for about one capture the dots were drawn off screen.
  - **The rule now:** the row moves to the new artboard once that corner is inside the canvas, and leaves the old one only when the old corner has left.
  - **The window test now asserts** the row is inside the canvas in every frame, with steps spaced as in this run. It failed before the fix.
- **The first contact sheet** cropped each capture where `ui.inspect` (read just after the screenshot) placed the row. During a camera flight that is a moment later, so some tiles looked empty. The sheets now show a fixed band across the canvas's top: what each screenshot shows.

## Not verified

- **A real agent's own session.** The MCP client was a script announcing Codex's name. No Codex, Claude Code or other agent ran these builds, so how each one calls `finish_working` (or doesn't) in practice wasn't observed.
- **`DELETE /mcp` from a real client, and `--attach` closing:** covered by `crates/mcp/tests/working.rs` and `control_server_tests.rs` only.
- **Two clients at once, live:** covered by window tests only.
- **Reduced motion and Animations Off live** (the static row for the whole run): window tests only.
- **The AccessKit announcements with a screen reader:** the node's text was read from `ui.inspect`; VoiceOver wasn't run.
- **Timings on an unloaded machine:** 16 GB of swap was allocated throughout. Two runs swapped during the measurement: the steps run (304 pages) and the ON half of the pixel comparison (2,188).
