# Idle repaints (Fix Z5, 23 Oct 2026)

**The question:** does the window keep repainting while nothing happens? The brief's bar: fix it only above about 2 repaints a second while nothing animates.

**The answer:** no fix was needed. Once what a run started has settled, the window paints once and then waits for the next input or timer: 0.1 a second, the one frame that starts each measurement.

## How it was measured

`crates/ui-egui/src/ai_idle_tests.rs` › `idle_repaints_stay_under_two_a_second`, in the test harness (the whole window drawn offscreen, 1440 × 900):
- **It emulates the event loop:** after each frame, the next one comes when the delay egui asked for has passed (at most 60 a second). When egui asks for no repaint within the 10 s, nothing more is painted.
- **The causes are egui's own** (`Context::repaint_causes`): the file and line of each request that painted the frame. A frame can have several.
- **Each case:** the first 10 s, then a steady 10 s.
- **The test is the regression check:** the steady rate must stay at or under 2 a second in all three cases, and the tooltip must still be open at the end.

Run it with `cargo test -p figcraft-ui-egui --lib -- panel::idle_tests --nocapture` to print the numbers.

## The numbers

| Case | First 10 s | Steady 10 s |
|---|---|---|
| The AI tab open, idle | 3 frames (0.3/s) | 1 frame (0.1/s) |
| The Design tab open, idle | 10 frames (1.0/s) | 1 frame (0.1/s) |
| A tooltip open on Undo run, after a run had ended | 2 frames (0.2/s) | 1 frame (0.1/s) |
| The 10 s right after that run ended, before the tooltip | 61 frames (6.1/s) | — |

**By cause:**
- **`lib.rs:1279`** is the autosave timer (every 30 minutes), which asks to be woken for its next checkpoint. It shows in most frames because it re-asks every frame, but it never wakes the window within 10 s. The one steady frame is the frame that starts each measurement.
- **`scroll_area.rs:753` and `:1524`:** the scroll bars fading after a tab switch, for a few frames.
- **`context.rs:538`:** egui's own first-frame setup.
- **After a run:** 61 frames in the first 10 s, all of them animation:
  - the camera's flight to the agent's frame (`camera_flight.rs:175`, 22 frames);
  - the new layer's reveal (`reveal.rs:471`, 45);
  - the working indicator fading out (`working_indicator.rs:233` and `:238`, 38 and 23);
  - the render worker (2) and the working log (1).
  These end by themselves, and the next 10 s are idle.
- **The tooltip** costs nothing once shown: egui draws it with the frame and asks for no repaint while the pointer is still.

## Not measured

- **The real window.** The harness has no MCP server, no file watcher and no platform event loop, so a repaint started from another thread (an MCP request, a file change on disk, a font loaded in the background) isn't counted here. Each of those is an event, not an idle loop.
- **The working banner and the card's bar while a run goes.** They animate on purpose: every 100 ms and every 50 ms. With reduced motion the banner's dots stand still and don't repaint.
