# The dots' gap between batches with Claude Desktop (Fix M0)

**21 Oct 2026.** The report, the hypotheses, the evidence, and what was and wasn't found.

## The report

An agent in Claude Desktop, connected through `npx mcp-remote@0.14.3`, announcing "local-agent-mode-scrim (via mcp-remote 0.14.3)", built a poster in six batches a few seconds apart, then called `finish_working`.
- **What the person saw:** the dots stayed on, except between batches, where they stopped for about 0.5 s and came back.
- **The brief's reading:** about a 400 ms fade out and a 160 ms fade in.
- **For contrast:** Stage I.1's live run, a script talking straight to the endpoint, had the dots in every frame.

## Result: not reproduced, and none of the four hypotheses is observed

Seven runs through the real `mcp-remote@0.14.3` bridge, on the current build, had the dots in every frame from the first batch to `finish_working`. The window logged no transition at any batch boundary. Below are the evidence, what each hypothesis predicts, and what remains open.

**No fix is shipped** (the brief: "Do NOT ship a fix for a cause you did not observe"). M1, M2 and M4 are not done. The diagnostics stay, so the next real session records what happens (see "What to send").

## The diagnostics added (`ee7b8d9`)

- **Settings ▸ Diagnostics, a `working` event for every transition of the working state:**
  - **The change:** `begin`: a client starts; `end`: a client stops and with it the run, and the dots fade out; `hold`: a client stops while the run carries on.
  - **The cause:** first request, finish_working, session closed (`DELETE /mcp`), grace period expired, or disconnected (an `--attach` connection closing).
  - **The client:** its name as announced and its session id cut to 8 characters.
  - **Never:** document content or tokens. `no_secrets_reach_the_events` covers the kind.
  - For example: "begin: local-agent-mode-scrim (via mcp-remote 0.14.3) (session 1c90825b…), first request; the dots are on".
- **`ui.inspect` › `agent.workingAlpha`:** the dots' opacity (0..1), so a fade between batches shows as a number.
- **The test:** `working_gap_tests.rs` drives a begin, an end on a session closing, a begin, a second client, a hold on `finish_working`, an end at grace expiry, and a disconnect, all on the request path the endpoint uses.

## The reproduction

The scripts are in `docs/scrim-ports/working-gap/`; `d3.py`, the control-channel helper, isn't committed (as for IE).
- **`bridge.py`:** spawns `npx -y mcp-remote@0.14.3 http://127.0.0.1:<port>/mcp` and speaks newline-delimited JSON-RPC to it over stdin and stdout, as Claude Desktop does:
  - `initialize` with clientInfo "local-agent-mode-scrim", `notifications/initialized`, `tools/list`;
  - six `batch` calls of 6, 12, 18, 24, 30 and 36 layers, 3, 5, 4, 3 and 5 s apart, on a blank file;
  - then `finish_working`.
- **What it records while it runs, over the control channel:**
  - a window capture with `ui.screenshot` every 100 ms (about every 115 ms in practice; 86 ms each);
  - `ui.inspect` every 25 ms (every 45 ms in practice), for the dots' opacity, whether the row is drawn, frame times and follow;
  - the diagnostics at the end.
- **`relay.py`:** a transparent HTTP relay between the bridge and the app. It logs each TCP connection opening and closing, and each request's method, `Mcp-Session-Id` and JSON-RPC method, with the response's status. It rewrites `Host` to the app's port, which the endpoint requires; nothing else.
- **`presence.py`:** per capture, the dots' orange pixels in the row's rectangle (as `ui.inspect` placed it).
- **The app:** the release build of `ee7b8d9`, on a fresh scratch home each run.

### The variants

1. **direct:** the bridge straight to the endpoint.
2. **relay:** through `relay.py`, for the wire log. The capture thread failed in this run (my script), so it has the wire log and the window's diagnostics only.
3. **agent:** after each batch, what the Desktop agent did between its batches (L3's record): `screenshot`, `inspect_document` and `lint_design`.
4. **tall:** a 1080 × 1920 poster, each batch further down it.
5. **both:** agent and tall.
6. **real:** a 1080 × 1350 poster built like one: text in Inter, Source Serif 4 and Geist at five sizes, ellipses and rounded rectangles; plus the agent's calls.
7. **artboards:** each batch on a new poster 1200 apart (as an agent duplicating its poster), so the row moves to a new artboard at every boundary.

### What they show

| Run | Captures with dots, first batch to finish | `ui.inspect` samples drawn at full opacity | Dots gone after finish | Working events | Longest frame (ms) | Swap-outs |
|---|---|---|---|---|---|---|
| direct | 214 / 214 | 455 / 455 | 249 ms | 2: begin, end | — | not recorded |
| agent | 219 / 219 | 467 / 467 | 200 ms | 2: begin, end | — | 0 |
| tall | 212 / 212 | 466 / 466 | 238 ms | 2: begin, end | — | 0 |
| both | 219 / 219 | 464 / 464 | 226 ms | 2: begin, end | — | 0 |
| real | 216 / 216 | 461 / 461 | 247 ms | 2: begin, end | 37 | 12,900 |
| artboards | 215 / 215 (see below) | 461 / 461 | 250 ms | 2: begin, end | 22 | 0 |
| relay | — (no captures) | — | — | 2: begin, end | — | 30,556 |

- **The only two working events of each run** are `begin` (first request) at the first batch and `end` (finish_working). No `end`, `hold` or `begin` at any boundary.
- **The opacity never drops below 1** between the first batch and `finish_working` (`workingAlpha`, every sample).
- **"Gone after finish":** the first capture with no orange in the row, 0.2–0.25 s after `finish_working`, when the 400 ms fade is about half done.
- **The artboards run:** `presence.py` found no orange in 7 captures, each just after a boundary, because the row was measured where `ui.inspect` placed it a moment after the screenshot, while the camera was flying.
  - By eye, every one of them shows the dots, on the previous artboard (`artboards-boundary-frames.png`). As built in Stage I.1, the row stays there until the new artboard's corner is in view.
  - So that run has dots in all 215 captures too.
- **The real poster's boundaries:** every 100 ms capture from −0.5 s to +1.0 s around batches 2–6 is in `boundaries-real-poster.png`. The dots are at full strength in all 72 tiles, and the wave moves on unbroken across each boundary.
- **Swapping:** two runs swapped (real: 12,900; relay: 30,556), so their timings are INVALID. Their behaviour counts still stand.

## The hypotheses, from the evidence

### (a) A new session per request, or a session closed after a request: not observed

- **The bridge's wire log** (relay run, `logs/relay/wire.jsonl`):
  - one `initialize` as "local-agent-mode-scrim (via mcp-remote 0.14.3)", and every `tools/call` after it on the same `Mcp-Session-Id`;
  - **no `DELETE /mcp`**, from the first request to `finish_working`.
- **What the bridge does do with connections:** it closes an idle TCP connection after about 4 s, and opens a new one for the next request (connections 1 → 3 → 4 → 5 at the 4- and 5-second gaps).
  - The endpoint ties nothing to a TCP connection: only `DELETE` ends a session (`figcraft_mcp::http`).
  - The window's diagnostics show no transition at those moments.
- **Claude Desktop's own log of the reported session** (`~/Library/Logs/Claude/mcp-server-scrim.log`; read for this record, not copied):
  - Desktop started `mcp-remote` at 17:38 local, after both apps were restarted at 17:37.
  - The "local-agent-mode-scrim" process sent one `initialize`, `notifications/initialized` and `tools/list`, then only `tools/call`. There was no second `initialize` and no reconnect.
  - The Scrim window's saved diagnostics agree for the night before: one connection per Desktop start, not one per call.
- **Also from both logs:**
  - **Claude Desktop starts two bridges:** "claude-ai (via mcp-remote 0.14.3)", which only lists tools, prompts and resources, and "local-agent-mode-scrim (via mcp-remote 0.14.3)", which calls the tools.
  - **Each bridge first initializes a probe session**, "mcp-remote-fallback-test", which it never uses or closes.
  - Neither makes requests that could end the working state.

### (b) Something at the end of a batch ends or restarts the indicator: not observed

- **No transition at any boundary** in the diagnostics, and the opacity stays 1 throughout.
- **The wave's phase** is a pure function of the clock and the artboard's id (`working_indicator::wave`, `t = now`). Nothing resets it; the sheets show it continuous.
  - A new artboard changes the wave's seed, so its pattern differs. In the artboards run that change happens when the row moves to the new poster, not as a fade.
- **What the code does at a boundary:** the reveal queue emptying, the flight ending and the follow target changing don't touch the working state. Nothing else reads the "last edit" any more (Stage I.1 removed the 0.6 s idle rule).

### (c) Frame drops or the degradation order touching the dots: not observed here

- **The longest frame was 37 ms** (the realistic poster, under swap), and the degradation step stayed 0.
- **Degradation never hides the dots** (`follow_agent::budget`, `working_indicator::paint`):
  - step 1 draws the still row instead of the wave;
  - step 2 stops the outline's pulse;
  - step 3 halves the reveal's cap.
  - A step is never taken back while agents work; the level resets only after 2 s of nothing animating.
- **One reading of the report that fits (c) differently:** "stopped … and came back" can describe a frozen window rather than a fade. While the UI thread executes a large batch, no frame is drawn, so the wave stops where it is and resumes.
  - On this machine, with these posters, no landing took longer than 37 ms, so this wasn't seen.
  - A heavier document (images, many fonts, effects), or a window macOS throttles while Claude Desktop is in front, could make it happen. Neither was tested.

### (d) The client's identity changes between requests: not observed

The session id and clientInfo were the same for every request, in the wire log, in the window's diagnostics and in Desktop's log.

## Not verified

- **The real Claude Desktop app wasn't driven.** The same bridge (`mcp-remote@0.14.3`, started with `npx` as Desktop starts it) was driven by a script. The agent's own timing and calls between batches were approximated from L3's record. The reported session's tool calls have no timestamps in Desktop's logs.
- **The reported session's window recorded no working events:** it ran the build from before `ee7b8d9`, which had no such diagnostics.
- **The Scrim window in the background:** in every run here the Scrim window was in front. Claude Desktop is in front while its agent works, and macOS can throttle a background app's frames. That wasn't tested.
- **A heavier document's landing frame:** see (c).

## What to send (asking the person to retest)

Retest in Claude Desktop with this build (`ee7b8d9` or later). Then open Settings ▸ Diagnostics and use Copy:
- **If the dots fade at a boundary,** the `working` events will say which client ended, when and why.
- **If the events show only one `begin` and one `end`, as here,** the state never ended, and what was seen was the window not drawing: a long frame, or macOS throttling it in the background.
- **A screen recording of the gap** would tell a fade (the dots dimming) from a freeze (the dots stopping mid-wave).
