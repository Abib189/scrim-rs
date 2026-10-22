# Fix Q4: a real agent's pauses at the default 30 s grace (22 Oct 2026)

**Claude Desktop itself was not driven.** `q4.py` spoke MCP over stdin and stdout, as Desktop does, to a real bridge process:
- **`npx -y mcp-remote@0.14.3`:** the client in the report;
- **Scrim's own `mcp-bridge`:** what Connect Claude Desktop writes since Fix P.

**The setup:**
- **The app:** the release build of `11adbfa`, with a scratch `HOME` and a scratch settings folder (`SCRIM_SETTINGS_DIR`, Scrim's rules 9). The app confirmed it (`app.settings-dir` → `override: true`). The grace was the default: nothing set it.
- **The build:** a poster in six batches (6–36 layers), with **8, 12, 15, 10 and 9 s** between them, then `finish_working`.
- **The real settings file** (`~/Library/Application Support/design.scrim.app/state.json`) had the same timestamp, size and checksum before and after both runs.
- **The scripts:** `q4.py` and `presence.py` (Fix M's, made scale-aware: one run's captures were 1×, the other's 2×). `d3.py` is the control-channel helper.
- **The logs:** `logs/npx`, `logs/scrim`.

| Run | Gaps (s) | Captures with dots, first batch to finish | `ui.inspect` samples drawn at full opacity | Working events | Dots gone after finish | Saved grace | Swap-outs |
|---|---|---|---|---|---|---|---|
| mcp-remote 0.14.3 | 8.0, 12.0, 15.0, 10.0, 9.0 | **498 / 498** | **1161 / 1161** | begin (first request), end (finish_working) | 0.17 s | 30, not chosen | 84,304 |
| scrim mcp-bridge | 8.0, 12.0, 15.0, 10.0, 9.0 | **491 / 491** | **1240 / 1240** | begin, end (finish_working) | 0.02 s | 30, not chosen | 0 |

- **Each run has two working events:** one `begin` at the first request, one `end` at `finish_working`. There is no `end` at any gap, so no "grace period expired", and no "grace too short".
- **"Gone":** the first capture after `finish_working` with no dot-coloured pixels in the row. The 400 ms fade takes them below that threshold well before it ends.
- **The captures** (`npx-mid-15s-gap.png`, 7.5 s into the 15 s gap; `npx-0.5s-after-finish.png`):
  - mid-gap: the dots over the poster, its outline, and "local-agent-mode-scrim (via mcp-… is working";
  - after the finish: none of them.

## Not verified

- **The real Claude Desktop app** wasn't driven, and neither was a real agent: its own timing and calls were stood in for by fixed gaps. **Please retest in Claude Desktop.** Your saved 5 s becomes 30 s when the new build first starts (no choice was recorded).
- **The window wasn't drawing for part of one gap.** In the `scrim mcp-bridge` run, no window capture could be taken for 6.2 s, from 24.1 s (inside the 15 s gap): a screenshot waits for the window to draw. The 25 ms `ui.inspect` samples carried on and show the working state on, at full opacity, throughout.
  - Most likely the window was behind another or hidden: real input has reached these test windows before (Fix L).
  - Those 6.2 s have no pixel evidence.
- **The npx run swapped** (84,304 pages): its timings are INVALID, including a 1.1 s pause between captures at 48.5 s. The presence counts still stand.
- **Gaps longer than 30 s,** and the AI-tab hint, live: tests only (`grace_hint_tests.rs`).
