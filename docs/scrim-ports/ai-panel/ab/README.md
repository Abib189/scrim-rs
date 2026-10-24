# Fix AB7: the AI panel's design states, from the real app

The seven states of the design's page "AI panel — A+B" (`../ab-design/`, exported from the design file with the scrim CLI), captured from the real app (a debug build of this tree) on a scratch HOME and settings folder, MCP on 7690, a stub `claude` (`crates/mcp/tests/stub/stub_agent.py`) first on its PATH. Both themes.

- `{theme}-{state}.png`: the app's AI panel, from its header to the window's bottom, at 1×.
- `side-{theme}-{state}.png`: the design's frame (left) and the app's capture (right), tops aligned.

## How they were made

1. `capture.py APP SCRATCH OUTDIR THEME WIDTH [MODE]` (MODE `states`, `quiet`, `fade` or `card`) drives the app like a person (`ui.input` clicks, typing, ⌘↩), once per theme at 336 px (A1 to A6) and once at 240 px (A7). It refuses to run unless the settings folder is a scratch one and MCP is not on 7680. The run is the stub's "batches 6 and slow 25": six batches of three rectangles, slowly, so A2 is caught while it works.
2. `crop.py SHOTS OUT` cuts each window shot to the panel, using the header's rectangle recorded at shot time.
3. `sidebyside.swift` (`swiftc sidebyside.swift -o sidebyside`) puts each next to its design export.

A2 to A6 were retaken after the last AB7 fix, in both themes; A1 and A7 after Fix AC2. Clicks can miss while another app is in front, so each step is retried until `ui.inspect` shows it took, and each shot waits for its file.

The window is 1440 × 900 points, so the panel below the header is about 720 high, where the design's frame is 900: everything at the bottom (the composer, the status line, the toast, the + menu) sits higher in the captures. That is the window, not the panel.

## The differences

**Data, not design** (the stub's run is not the design's poster):
- The prompt and the reply's words; "18 layers changed" and "Created 18 layers." for the design's 43 and its four counts; "Done · 28 s" for "Done · 1:47"; one step ("Steps 1", "Created Rectangle ×18") for five.
- A2 "Steps 0 of 1" for "3 of 5": the stub's work is one step, under way. The agent has no plan, so there is no total ahead (decision in `docs/design/ai-panel.md`).
- A6 "Attach image" is off: the stub agent can't take an image.
- A5 and A6: the header's Follow button has the amber paused dot (Fix N): following was paused, most likely by the capture's own clicks and selection. The status line still says "Following is on", as decided (paused counts as on; the button shows the pause).

**Intended** (earlier decisions, kept):
1. **The header's title** is the conversation's: "New chat" before the first message, then the first prompt cut with "…" (Fix Z3). The design shows the document's name.
2. **The Follow icon** is Scrim's crosshair (`AiIcon::Crosshair`, decision 11 in `docs/design/ai-panel.md`), where the design has a ring.
3. **A rule under the header**, as the panel has had since the redesign; the design has none.
4. **A2: the working banner** under the header ("Claude Code is working 0:06", Fix X4). The design shows only the card.
5. **The order of the transcript:** the card is under your message, and the agent's reply follows it, as the run happened; the design puts the reply above the card.
6. **The agent's name** ("Claude Code", SemiBold 12) over its reply, and **the cost line** under it ("0:28 · Billed by your agent's plan.").
7. **The placeholder** is "Describe a change…" in every state; the design varies it ("Ask Claude Code…", "Ask for a tweak while it works…", "Ask for a change…").
8. **The bar while working** is indeterminate, a short moving segment; the design shows a part-filled bar. No percentage: the agent has no plan to measure.
9. **A5's heading** reads "Undone · 28 s", the run's length, where the design shows "17:52".
10. **The composer's accent edge** in A2 and A3: the composer keeps focus after ⌘↩ sends, and a focused field has the accent edge. The capture's click on the card in A4 takes the focus, so A4 to A6 have the plain edge.
11. **A7: the selection chip** reads "Selected · Poster —…" at 240 px, cut by AA3's name rules; the design fits the whole name.

**Fixed in AB7** (seen in these side-by-sides, then changed and recaptured):
- The + menu sat over the composer; it is now 8 above the status line, its left edge at the composer's (`the_plus_menu_clears_the_status_line`).
- The selection's suggestion rows were inset 16 more than the composer; they now span its width, their words cut with "…" (`ai_suggest_tests.rs`).
- A5's Undone toast was inset 16 more than the composer each side; it is now the composer's width (`undo_run_shows_the_toast_and_its_redo_puts_the_run_back`).
- A5's Select layers, off after Undo run, dimmed only its label; its edge is now dimmed too, as in the design.
- A4's chevron pointed down while the steps are open; it points up, as in the design (`the_steps_open_and_close_and_start_closed`).

**Fixed in AC2** (A1 and A7 retaken in both themes):
- A1's empty line broke after "Follow" at 336 px; it now breaks after "canvas.", as in the design: the line wraps in the design's 248 px box, or the panel less 32 when narrower.
- A1's block sat about 17 px higher than the design's rule puts it; it is now centred between 16 under the header and 14 above the suggestions' label (the design's "Empty state" frame), with the design's 12 and 12 between disc, heading and line (they were 16 and 8). `the_empty_state_wraps_and_sits_as_in_the_design`.

**Not intended, left as they are:**
- **A7: one word.** At 240 px the design breaks "…this / canvas. Follow keeps the camera / on its edits."; the app's middle row is a little wider than 208 px in egui's layout, so "camera" goes to the third row.
- **The block's height on the page** differs from the design's because the window is shorter (723 px under the header against 852); its place relative to the header and the suggestions is the design's.

## AC1: the transcript's fade

`{theme}-fade-top.png` and `{theme}-fade-bottom.png`: three runs (`capture.py … 336 fade`), then the transcript scrolled to the top (975 px more below) and to the bottom. At the top, the 12 px fade (`ui.inspect` › `fade`, y 744 to 756) dissolves the last visible line, the reply's cost line, into the panel just above the status line; at the bottom there is none. **It reads in both themes; nothing changed.**

Seen on the way: only the latest run has a card. The two earlier runs show their progress lines ("Agent: made 3 layers", one per batch) above their replies, as the transcript did before the card (`ai_run_card.rs` keeps one `last_run`). Not changed here.

## AC3: the card at 280 px

`{theme}-card-280-done.png` and `{theme}-card-280-undone.png`, each beside the design's A3 or A5 (`side-…`, `capture.py … 280 card`). Below 300 px Select layers and Undo run (or Redo run) stack, each the card's full width (`ui.inspect` › `card.stacked`: true in all four). After Undo run, Select layers is off with its edge and label dimmed, as in A5.

**Against the design's intent:** the design has no card narrower than 336 px, so the stacking is the app's own rule (Fix Y2, decision in `docs/design/ai-panel.md`). At 280 px the design's side-by-side pair would still fit (each half about 105 px, "Select layers" needing about 103 with its 12 px padding), so the app stacks where the design's layout wouldn't have to. Reported, not changed.
