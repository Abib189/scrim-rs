# Fix AA: the AI panel with its real fonts, offscreen captures (23 Oct 2026)

`offscreen_tests.rs` › `ai_panel_aa_captures` (run with `SCRIM_AI_AA_DIR` set) is Fix Z's sequence with the app's real fonts: the harness installs them as the app does (`theme::install_fonts`). Each file is the right panel cut from the window, from the tab row down. They were made in the test harness, not taken from a real window.

**Fix AA1, the weights** (`aa-{theme}-{width}-{state}.png`, light and dark, at 336 and 240 px):
- **`1-done`:** the ended run, its card, the Markdown reply and the composer.
- **`2-undone`:** the card after Undo run.
- **`3-composer`:** Codex chosen.

## Do the weights read?

**Yes.** These read as SemiBold, against the Regular body:
- the header title;
- "Done · 42 s" and "Undone · 42 s";
- "Claude Code" over the reply;
- "**Poster — Welle**" in the reply.

"*five*" is a real italic, not a slanted regular. In Z's captures, made before this fix, all of them drew at the body's weight.

**What still looks off:**
- **Inline code at 240 px dark:** `#FF6A00` sits on `t.field`, which is close to the dark panel, so its box is faint.

**Fix AA3, the agent chip at 240 px** (the captures were made again with it):
- **`aa-*-240-3-composer`:** Codex is shown whole, without a chevron, with 8 px padding, and the image is in the agent menu ("Attach image…"). Before AA3 the chip was only "…".
- **`aa-*-240-1-done`:** with Claude Code (no image) the chip reads "Claude C…".
