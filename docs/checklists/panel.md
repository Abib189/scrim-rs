# Right panel: mode switch and tabs (Stage B)

The plan: `docs/design/panel-redesign.md`. Screenshots before and after: `docs/design/panel-redesign/`.

| Check | How | Covered by |
|---|---|---|
| The header shows Design \| Animate and the zoom menu, then the tabs Design, Prototype, Code and AI | Open a file and look at the right panel | `crates/ui-egui/src/panel_view_tests.rs` › `clicks_switch_tabs_and_modes_and_never_touch_the_document` |
| Clicking a tab or a mode runs `panel.tab.*` / `panel.mode.*`; switching never edits the file or adds an undo step | Click each tab and both modes, then look at Edit ▸ Undo | same test; `crates/engine/src/cmd/panel_tests.rs` › `the_choice_is_a_preference_and_nothing_touches_the_document` |
| Animate is a placeholder: a note, Back to Design, no tabs | Click Animate | `clicks_switch_tabs_and_modes_and_never_touch_the_document`; `screen_readers_get_the_mode_switch_and_the_tabs` |
| Agents switch the panel with the same commands, over MCP and the control channel; `ui.inspect` reports `rightTab` and `panelMode` | `run_command {command: "panel.tab.code"}` | `agents_switch_them_with_the_same_commands`; `crates/mcp/tests/panel.rs` |
| Each window keeps its own mode and tab; new windows and the next start open with the last chosen | Switch to Code, open File ▸ New window, then quit and start again | `the_next_start_opens_with_the_last_choice`; `windows.rs` copies the window's mode (manual for the new-window case) |
| Screen readers: "Design mode" and "Animate mode" toggles; the tabs by name, the current one selected; the panel's region is named after what it shows | VoiceOver or Narrator on the panel | `screen_readers_get_the_mode_switch_and_the_tabs` |
| The Code tab shows the selected layer's code with the language and unit menus and Copy; without a selection it says what to do | Select a layer, open Code | `clicks_switch_tabs_and_modes_and_never_touch_the_document` (Copy CSS) |
| Dev Mode (Inspect) is unchanged and leaves the remembered tab alone | Shift+D twice | `agents_switch_them_with_the_same_commands` |
