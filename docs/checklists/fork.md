# The fork's own guarantees

| Check | How | Covered by |
|---|---|---|
| No ArtCraft marks, links or name outside "based on FigCraft by the ArtCraft team" | Open About: it says Scrim, links to github.com/Abib189/scrim-rs and credits FigCraft in plain text. Help ▸ Help opens the fork's README | `cargo xtask brand`; `xtask/src/brand.rs` › `links_are_refused_everywhere_and_the_name_only_in_shipped_text` |
| Crash and startup messages send reports to the fork's issues | Read the "window can't open" message (`dialogs::window_failed`) | `apps/figcraft/src/startup_tests.rs` › `the_message_carries_the_error_and_where_to_report_it` |
| No source file over 1,000 lines; frozen files never grow | Not manual | `cargo xtask size`; `xtask/src/size.rs` › `new_files_over_the_limit_and_frozen_files_that_grow_are_refused` |
| On macOS, a dangling link inside an automation write root says "leads nowhere", not "outside" | `figcraft-cli mcp --automation-write-root $TMPDIR/x`, then `save_file` to a dangling link inside it | `crates/engine/src/file_access_tests.rs` › `links_that_lead_out_of_a_root_are_outside` |
