# Fix N3: the "Follow agent" chip goes when the work ends (21 Oct 2026)

**The setup:**
- **The app:** the release build of this commit, on a fresh scratch `HOME`, window 1440 × 900. Captures are the window's own pixels at 1× (`ui.screenshot`).
- **The agent:** the real `mcp-remote@0.14.3` bridge, spawned with `npx` and spoken to over stdin and stdout as Claude Desktop does (`n3.py`, its client from Fix M's `bridge.py`). It announced "local-agent-mode-scrim" and talked to the app's MCP endpoint over HTTP.
- **The person:** the control channel only set the starting state, turned the wheel over the canvas once (`ui.input`), captured every 100 ms and read `ui.inspect` (`agent.follow`, `agent.chip`, the view).
- **Conditions:** load average 13.9, no swap-outs during the run (`conditions.json`).

## The run (`log.json`; times from the start of the script)

| Time | What happened | Seen |
|---|---|---|
| 4.46 s | Batch 1: the "Poster" artboard and 8 rectangles | following; the camera goes to the poster |
| 7.52 s | Batch 2 | following |
| 9.02 s | **The person scrolls the canvas** | **9.10 s: paused, the chip shown** (`01-paused-chip-shown.png`) |
| 11.08 s | Batch 3, while paused | the camera stays where the person left it: (540, 1014) |
| 13.62 s | **`finish_working`** | **13.66 s:** following, the chip fading (opacity 0.31, `03-…`). **13.78 s: no chip** (`04-…`): gone within 0.16 s of the call |
| 13.62–16.64 s | Nothing | the camera stays at (540, 1014), with no fly back |
| 16.64 s | **A second build: "Poster 2" at x 4000, 8 rectangles; no other input** | **16.78 s:** the camera on its way (4054, 716); **16.90 s:** there (4540, 675) (`05-second-build-followed.png`) |

`sheet-pause-and-finish.png` has every capture around the scroll and around `finish_working`.

**Settings ▸ Diagnostics** read, in order:
1. "begin: … first request";
2. "following paused: the person's press, scroll or pinch on the canvas";
3. "end: … finish_working";
4. "**follow pause cleared: the agent finished**";
5. "begin: … first request" (the second build);
6. "end: … finish_working".

## Pixels: the last capture before `finish_working` (f0080) and one after the fade (f0095)

| Region | Changed pixels | Why |
|---|---|---|
| Top bar (y 0–40) | **0** | — |
| Left panel (x 0–296) | **0** | — |
| Canvas | 5,066 | The chip (2,530 in its box), and the agent's outline around "Poster" fading because the run ended (Stage I.1). **The camera is unchanged**: the same view before and after. `diff-canvas-top-before-after.png` |
| Right panel (the AI tab) | 34,291 | Its status: "… is working" ends, and the paused row ("Resume", "Paused: you moved the camera or edited") becomes "Following local-agent-mode-scrim …". The working line going away moves every row below it up one line, which is most of this count. `diff-ai-tab-before-after.png` |

So the changes outside the canvas are all in the AI tab, and the canvas's changes are the chip and the run's own end. Nothing else changed.

## Not verified

- **Claude Desktop itself wasn't driven:** the same bridge process was, by a script.
- **The AccessKit tree in the release build:** the chip leaving the tree is covered by `follow_end_tests.rs` only. VoiceOver wasn't run.
- **The grace period's end and a disconnect in the release build:** both are covered by `follow_end_tests.rs` only.
- **Two clients live:** tests only.
- **Animations Off or reduced motion live:** tests only.
