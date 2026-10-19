# SVG: refused, never sanitised (Port 8)

The report: `docs/scrim-ports/svg.md`.

| Check | How | Covered by |
|---|---|---|
| Every file Scrim refuses as unsafe is refused with the same reason (23: script, handlers, entities, `@import`, external links) | Open such an SVG (File ▸ Open, or paste): an error naming the reason, nothing imported | `crates/svg/src/import/scrim_cases_tests.rs` › `every_file_scrim_refuses_as_unsafe_is_refused_and_its_gaps_are_closed` |
| The 11 gaps Scrim's review listed are closed (stylesheet PIs, SYSTEM DTDs, encoded schemes, SMIL links, relative files, `foreignObject`, `<svg/onload>`, unbounded nesting) | Not manual | the same test |
| Every safe file Scrim accepts imports (12), and so does every vector Scrim generated (12) | Open a Lucide or Material icon | the same test; `scrims_generated_vectors_import` |
| A linked file is never read: the SVG is refused before parsing | Not manual | `crates/svg/src/import/structure_tests.rs` › `linked_files_are_never_read`; `budget_tests.rs` › `linked_files_and_other_mime_types_stay_unresolved` |
| Path data an agent passes parses as Scrim's does (11 read to the same shape, 6 refused, numbers after `Z` included) | Not manual | `crates/engine/tests/scrim_paths.rs` |
