# `.fig` import: Scrim's security design and fidelity (Port 4)

The report: `docs/scrim-ports/fig-import.md`.

| Check | How | Covered by |
|---|---|---|
| Every hostile input in Scrim's case file is refused, or bounded by the layer budget; none panics or runs away | Open a damaged `.fig` (File ▸ Open): an error, never a crash | `crates/format/src/fig/security_tests.rs` › `scrims_hostile_inputs_are_refused_or_bounded_never_a_panic` |
| An archive with an unsafe entry name is refused before anything is read; honest archives open | Not manual | `security_tests.rs` › `unsafe_entry_names_are_refused_before_anything_is_read` |
| Bytes after the message, or a message without layers, are refused with the reason | Not manual | `security_tests.rs` › `bytes_after_the_message_and_a_message_without_layers_are_refused` |
| Deep nesting (20,000 frames) and 100,000 layers import without overflow, within the budget | Not manual | `security_tests.rs` › `deep_nesting_never_overflows_the_stack`, `a_hundred_thousand_layers_import_within_the_budget` |
| Files over 200 MB and archives of more than 5,000 entries are refused | Open one | `crates/format/src/fig/mod.rs` (`MAX_FILE`, `MAX_ENTRIES`); manual only for the 200 MB file (too large for a unit test) |
| The fixtures and a real file still open with their layers | Open `docs/scrim-transfer/fixtures/fig/*.fig` and a real Figma file | `cargo xtask fidelity` (opens all five fixtures); the real sample by hand (753 layers) |
