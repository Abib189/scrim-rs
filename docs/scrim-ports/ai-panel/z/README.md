# Fix Z: the AI panel after a run, offscreen captures (23 Oct 2026)

`offscreen_tests.rs` › `ai_panel_z_captures` (run with `SCRIM_AI_Z_DIR` set) draws the whole window offscreen, with Ridgeline open and the AI tab showing. Each file is the right panel cut from that window, from the tab row down. They were made in the test harness, not taken from a real window.

**What they show:** light and dark, at the design's 336 px and at 240 px, the panel's narrowest.
- **`1-done`:** a run that has ended. It shows:
  - a 48-character title cut with "…" and the chevron after it (Z3);
  - the card's rows by name, with the agent's own batch label ("Lay out twenty bars in a sine wave", Z1);
  - the agent's reply drawn from its Markdown: bullets, italic, inline code, a numbered list and a code block (Z2);
  - the round Selection chip (Z6).
- **`2-undone`:** the same card after Undo run (Z7). It reads "Undone · 42 s", with the rows dimmed on empty rings, Select layers off and Redo run in Undo run's place.
- **`3-composer`:** Codex chosen, so the image chip shows next to Selection (Z6).

Names: `z-{theme}-{width}-{state}.png`.

## What looks right, and what doesn't

- **Right:**
  - **The title:** cut with "…" at both widths, its chevron 6 after it, and the buttons in place.
  - **The rows:** they say what changed, and a long row is cut with "…" at 240 px.
  - **The reply:** no `**`, backtick or `-` is left as text; bullets and numbers hang, and the code block wraps inside its box.
  - **The undone card:** it reads as undone.
- **Not right, or not like the real app:**
  - **At 240 px with Codex chosen, the agent chip is only "…".** The two round chips leave it 44 px, which is its padding and chevron alone. This was already so before Z6, because at 240 px both chips were already round. A fix could drop the chevron or the padding below a width; it isn't in this change.
  - **Bold looks no heavier than the body here,** and neither does the card's heading. The harness draws with egui's default font, not Inter (as the X captures note), so how bold reads in the real app isn't shown by these.
  - **"Undo is one step" stays on the undone card.** The brief doesn't change the note; it could read "Redo is one step" there.
  - **The fade (Z4)** doesn't show: the transcript is at its bottom in every capture.
