# Fix X6: the rebuilt AI panel, offscreen captures (23 Oct 2026)

`offscreen_tests.rs` › `ai_panel_x_captures` (run with `SCRIM_AI_X_DIR` set) draws the whole window offscreen, with Ridgeline open and the AI tab showing. Each file here is the right panel cut from that window, from the tab row down. They were made in the test harness, not taken from a real window.

**What they show:**
- **Themes:** light and dark.
- **Accents:** the default orange (`orange`), a very pale one, `#FFE066` (`pale`), and a very dark one, `#1F3BFF` (`blue`).
- **Widths:** the design's example width, 336 px, and 240 px, the panel's narrowest.
- **States:**
  - **Idle** (`1-idle`): the empty state and the status line.
  - **Mid-run** (`2-mid-run`): the banner and your message. The card has three done rows (Create frame, Rectangle ×20, Text ×2) and the active one (Set properties), the indeterminate bar, and Undo run held until the run ends. The agent's edits came over the app's MCP route.

Names: `x-{theme}-{accent}-{width}-{state}.png`.

## What looks right, and what doesn't

- **The layout matches the design** in both themes:
  - the header, with Follow on in the accent's tint;
  - the banner, the right-aligned bubble and the card;
  - the status line, and the composer with its chips and Send/Stop.
- **The accent follows everywhere it should:** the Follow icon, the working dots, the active ring, the bar and Send.
  - With `#1F3BFF` on dark, the marks are lightened (the accent rule) and Stop's square is white.
  - With `#FFE066` on light, the marks and Send's fill are darkened to an olive. Every role keeps its contrast (Fix X5's test), but this is the one look that may not be what a person picking a pale yellow expects.
- **At 240 px** nothing is clipped.
  - The agent chip reads "Clau…", and Selection becomes a round icon chip (Fix X3).
  - Since Fix Y2 the card's buttons stack at 240 px, each the full width, so "Select layers" has room. The 240 px mid-run captures were made again for it.
  - Your message wraps to four lines.
- **Not like the design:**
  - **The font:** the harness draws with egui's default font, not Inter. *(Corrected by Fix AA1: the real app did too. Its UI took Inter only from the craft-fonts build input, which these builds don't have, so SemiBold was the regular face. Since AA1 both use the bundled Inter; see `../aa/`.)*
  - **Undo run while the run goes:** these captures show it held (dimmed), as Fix X4 decided. The design's mock shows it ready.
