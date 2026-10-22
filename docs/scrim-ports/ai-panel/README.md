# Fix T4: the AI panel, captured (22 Oct 2026)

**Offscreen:** the screen was locked, so no window could be captured. These come from the offscreen renderer: `offscreen_tests.rs` › `ai_panel_screenshots`, run with `SCRIM_AI_SHOTS_DIR=… cargo test -p figcraft-ui-egui --lib ai_panel_screenshots -- --ignored`.
- **The panel's width:** 240 px, which is both its default and its minimum.
- **The scene:** the Ridgeline example.
- **The run under way** is a scripted one in the test platform: no agent ran.

| Capture | Light | Dark |
|---|---|---|
| Empty: header, chat bubble, composer | `ai-light-1-empty.png` | `ai-dark-1-empty.png` |
| The popover: search, "This document \| All documents", the list with a check | `ai-light-2-popover.png` | `ai-dark-2-popover.png` |
| A run under way (Stop, "Claude Code is working · 0:08") | `ai-light-3-running.png` | `ai-dark-3-running.png` |
| The grace hint's line, pending (Open the setting, ×) | `ai-light-4-hint.png` | `ai-dark-4-hint.png` |
| Settings ▸ Agents (Connections first) | `ai-light-5-settings-agents.png` | `ai-dark-5-settings-agents.png` |

The captures were read with `scripts/ocr-captures.swift` before they were committed: no paths, names or addresses.

**Not verified on a real window:**
- the panel in the app's own window;
- hover and tooltips;
- a real agent run;
- the image button's file dialog;
- conversations kept across a real restart. `services_tests.rs` checks the file is written and read back.
