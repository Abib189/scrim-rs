# Stage I verification (I4, 21 Oct 2026): follow the agent, the reveal, the working indicator

**The setup:**
- **The app:** real window captures from the release build at `1dde4c7`, `ui.screenshot`, 2880 × 1800 px at 2×. A scratch HOME, the dark theme, and the AI tab open.
- **The agent:** every agent edit went over the app's **real MCP endpoint** (`POST /mcp` on 127.0.0.1:7681, the fallback port, since another Scrim held 7680), as an MCP client named `claude-i4`. It sent `run_command frame.create`, then the `batch` tool.
- **The control channel** only captured the window, set the starting camera and played the person's input (a wheel turn, a click on the chip).
- **The script and the full logs:** `runs.json` holds every capture's time, camera and `ui.inspect`'s `agent` state.
- **The pixel checks** compare the full-size originals. The frames kept in `frames/` are downscaled to 1×.

## Sheet 1: the poster build (`sheet-1-poster-build.png`)

A blank file, the camera at (−9000, −6000), 50%. The agent creates "Poster" (600 × 800), then 15 layers in one batch (two rules, four dots, three blocks, a title and four lines of text): 16 layers in all. Times are from the frame's `create`.

| Capture | t | Camera | Entering | Dots |
|---|---|---|---|---|
| `a00-start` | — | (−9000, −6000) | 0 | — |
| `a00` | 0.04 s | flying (−7246, −4793) | 15 queued | — |
| `a01` | 0.15 s | flying (−500, −150) | 15 | — |
| `a02` | 0.27 s | **(300, 400): the poster centred**, the zoom kept (it fits) | 12 | 20 columns + outline |
| `a03` – `a06` | 0.37 – 0.67 s | there | 9, 7, 4, 2: one after another, in creation order | 20, waving |
| `a07`, `a08` | 0.77, 0.87 s | there | 0: all in | 20 |
| `a09`, `a12` | 0.97, 1.27 s | there | 0 | gone (faded 400 ms after 600 ms idle) |

The layers fade in one after another **and** the dots wave above the artboard in the same frames (`a02`–`a06`). The AI tab reads "Following Claude · Poster".

## Sheet 2: paused, off screen, Resume (`sheet-2-paused-offscreen-resume.png`)

| Step | Result |
|---|---|
| **A wheel turn over the canvas 0.12 s into a flight** to a new frame | The flight stopped at (−3665, −2509), short of its target, and following **paused**: the "Follow agent" chip at the top of the canvas (`b01-paused-chip`). |
| **Paused, the agent builds in view** ("In view", 11 layers) | Still revealed, with the dots: 11 → 8 → 5 → 2 → 0 entering over 0.6 s (`c00`–`c08`). |
| **Paused, the agent builds 16 layers off screen** ("Far", x = 30000) | No entrance: `revealing` 0. Scrolled to (`d01`), it is drawn final. **0 px differ** inside the artboard between `d01` and 1.5 s later (`d02`), so nothing was half-faded. |
| **The chip clicked** | `following` again; the camera flew to the latest work, "Far" (`e00`–`e02`, `e99-resumed`). |

## Sheet 3: Animations Off (`sheet-3-animations-off.png`)

The same poster build in a fresh app with Settings ▸ Animations ▸ Off:
- **A jump cut:** the camera is at (300, 400) in the first capture.
- **No entrance:** the layers appear final at once.
- **The static dot row** while the agent works; it is gone after idle.

| Check | Result |
|---|---|
| **Off vs On, the same moment mid-reveal** (`a02`–`a07`) | Differences only inside the canvas: **0 px outside it**. That covers the panels, the AI tab's status ("Following Claude · Poster" in both) and the tab bar. |
| **The final frames, Off vs On** | 163 px differ, at most 16/255 per channel, in a few rows inside the poster. These are the renderer's path-dependent edge pixels: the camera flew in one run and jumped in the other, the effect D3 recorded. After a fresh render of the same view in both runs (zoom 102% and back), the finals are **identical: 0 px**. |

## Frame times on the 782-layer Fixture

**The run:** the Fixture rebuild, with the camera on "Poster — The Moon Is Late". After the window had been idle for 2 s, the agent added 15 layers into that artboard in one `batch`. `ui.inspect` was sampled every ~50 ms. This is the release build on an otherwise idle machine; nothing was measured under swap.

| Run | Batch reply | The landing frame (the batch itself) | Revealing samples (layers entering) | Indicator samples | Frame (median / max) | Canvas render (median / max) | Degradation step reached |
|---|---|---|---|---|---|---|---|
| 1 | 0.106 s | 159 ms | 13 (14) | 17 | 1.9 / 3.9 ms | 2.8 / 3.2 ms | **0: none needed** |
| 2 | 0.108 s | 161 ms | 13 (14) | 18 | 1.9 / 3.2 ms | 3.0 / 3.1 ms | 0 |
| 3 | 0.098 s | 152 ms | 12 (14) | 17 | 1.9 / 2.5 ms | 2.7 / 5.0 ms | 0 |
| 4 | 0.105 s | 157 ms | 13 (14) | 17 | 2.1 / 4.2 ms | 3.0 / 3.1 ms | 0 |
| 5 | 0.117 s | 171 ms | 13 (14) | 18 | 2.1 / 3.3 ms | 3.1 / 3.7 ms | 0 |

**Read the table this way:**
- **Fast enough not to degrade:** with the reveal and the dots running, frames stay around 2 ms and the canvas re-renders only the revealing layers' areas in about 3 ms. No degradation step was needed. The steps (static row, then no outline pulse, then a 0.6 s cap) are covered by `under_load_the_steps_come_in_order_and_the_fade_stays`.
- **The landing frame:** the one long frame (150–170 ms) is the engine running the 15-command batch on a big file, before any reveal code runs. The reveal waits for it rather than being cut short.

**What I4 found and fixed on the way** (each is its own commit, with a regression test):
- **`459c95a`:** work arriving from far away was parked on the canvas's far edge, under the toolbar; it is now centred. The AI tab's status was cut to "…".
- **`24b163d`, `9585473`:** the landing's own long frame finished the reveal at once, so on the Fixture it never played; it is now re-timed.
- **`8ba781e`:** an idle window draws no frames, and the gap before its next frame was read as a hidden window, so the first edit after a quiet spell never revealed. Hidden is now marked where requests are answered while hidden.
- **`1dde4c7`:** one slow render stayed in `render_ms` and stepped the degradation all the way down; only new renders count now.

## Not verified

- **Not with Claude Code itself:** the MCP client was this script, speaking the real HTTP endpoint. "Claude" in "Following Claude" comes from its client name, `claude-i4`.
- **No live captures of** the deleted-layer fade, the move tween, the undo, redo and file-open skips, the 400 ms coalescing across separate tool calls, or the system's reduced motion. All of these are covered by tests only.
- **The degradation steps live:** the Fixture never needed them.
- **A real screen reader** (VoiceOver) reading the polite "Claude is working on Poster" status: it is checked in the AccessKit tree only.
- **The window-hidden path live** (requests answered while hidden draw final): it is covered by tests only.
- **The light theme, a second window, and Windows and Linux.**
- **The chip's and the dots' contrast:** measured from tokens in tests, not from pixels.
